// Local document search: Markdown, text, PDF (pdftotext), Word/ODT (via unzip) in a folder.
import { readFileSync, readdirSync, statSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { join, relative, extname, resolve } from "node:path";
import { homedir } from "node:os";
import { args, settings, fail, which } from "./common.mjs";

const roots = (settings.folders || "").split(",").map((s) => s.trim().replace(/^~/, homedir())).filter(Boolean);
const [cmd, ...rest] = args;
const EXT = [".md", ".txt", ".pdf", ".docx", ".odt", ".rst", ".org", ".html"];
const walk = (d, n = 0) => n > 8 ? [] : readdirSync(d).flatMap((x) => { if (x.startsWith(".") || x === "node_modules") return []; const p = join(d, x); try { const s = statSync(p); return s.isDirectory() ? walk(p, n + 1) : EXT.includes(extname(x).toLowerCase()) && s.size < 50e6 ? [p] : []; } catch { return []; } });
function text(p) {
  const e = extname(p).toLowerCase();
  if (e === ".pdf") { if (!which(["pdftotext"])) return ""; return spawnSync("pdftotext", ["-layout", p, "-"], { encoding: "utf8", maxBuffer: 64e6 }).stdout || ""; }
  if (e === ".docx" || e === ".odt") return (spawnSync("unzip", ["-p", p, e === ".docx" ? "word/document.xml" : "content.xml"], { encoding: "utf8", maxBuffer: 64e6 }).stdout || "").replace(/<\/w:p>|<\/text:p>/g, "\n").replace(/<[^>]+>/g, "");
  return readFileSync(p, "utf8");
}

switch (cmd) {
  case "connect": case "status": {
    if (!roots.length) fail("Add one or more folders (comma separated).");
    const n = roots.reduce((a, r) => { try { return a + walk(r).length; } catch { fail(`Folder not found: ${r}`); } }, 0);
    console.log(`${n} documents in ${roots.length} folder(s)${which(["pdftotext"]) ? "" : " - install poppler-utils for PDF search"}`);
    break;
  }
  case "search": {
    const words = rest.join(" ").toLowerCase().split(/\s+/).filter(Boolean);
    const hits = [];
    for (const r of roots) for (const p of walk(r)) {
      const lines = text(p).split("\n");
      const m = lines.map((l, i) => [i + 1, l]).filter(([, l]) => words.every((w) => l.toLowerCase().includes(w)));
      if (m.length) hits.push(`${p}\n${m.slice(0, 4).map(([i, l]) => `  ${i}: ${l.trim().slice(0, 200)}`).join("\n")}`);
      if (hits.length >= 25) break;
    }
    console.log(hits.join("\n\n") || "No matches.");
    break;
  }
  case "read": console.log(text(resolve(rest[0])).slice(0, Number(rest[1] || 40000))); break;
  case "list": for (const r of roots) console.log(walk(r).map((p) => relative(r, p)).join("\n")); break;
  default: console.log(`Usage:
  search <words>        lines containing all words (md, txt, pdf, docx, odt, html)
  read <file> [chars]   full text of a document
  list                  all documents`);
}
