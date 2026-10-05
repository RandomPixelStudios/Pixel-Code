// Obsidian vault (a local folder of Markdown notes).
import { readFileSync, writeFileSync, appendFileSync, readdirSync, statSync, mkdirSync } from "node:fs";
import { join, relative, dirname } from "node:path";
import { homedir } from "node:os";
import { args, settings, fail } from "./common.mjs";

const vault = (settings.vault || "").replace(/^~/, homedir());
const [cmd, ...rest] = args;
if (cmd && cmd !== "help") { try { if (!statSync(vault).isDirectory()) throw 0; } catch { fail("Set the path of your Obsidian vault."); } }
const walk = (d) => readdirSync(d).flatMap((n) => { if (n.startsWith(".")) return []; const p = join(d, n); return statSync(p).isDirectory() ? walk(p) : n.endsWith(".md") ? [p] : []; });
const note = (n) => join(vault, n.endsWith(".md") ? n : `${n}.md`);

switch (cmd) {
  case "connect": case "status": console.log(`Vault ${vault} (${walk(vault).length} notes)`); break;
  case "list": console.log(walk(vault).map((p) => relative(vault, p)).join("\n")); break;
  case "search": {
    const q = rest.join(" ").toLowerCase();
    for (const p of walk(vault)) {
      const lines = readFileSync(p, "utf8").split("\n");
      const hits = lines.map((l, i) => [i + 1, l]).filter(([, l]) => l.toLowerCase().includes(q)).slice(0, 3);
      if (hits.length || p.toLowerCase().includes(q)) console.log(`${relative(vault, p)}\n${hits.map(([i, l]) => `  ${i}: ${l.trim()}`).join("\n")}`);
    }
    break;
  }
  case "read": console.log(readFileSync(note(rest[0]), "utf8")); break;
  case "write": mkdirSync(dirname(note(rest[0])), { recursive: true }); writeFileSync(note(rest[0]), rest.slice(1).join(" ").replace(/\\n/g, "\n")); console.log("Written."); break;
  case "append": appendFileSync(note(rest[0]), "\n" + rest.slice(1).join(" ").replace(/\\n/g, "\n")); console.log("Appended."); break;
  default: console.log(`Usage:
  list | search <text> | read <note> | write <note> <text> | append <note> <text>   (\\n = new line)`);
}
