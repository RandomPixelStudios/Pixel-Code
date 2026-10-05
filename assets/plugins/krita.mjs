// Krita from the command line (export .kra and other files).
import { args, settings, fail, which, passthrough } from "./common.mjs";
import { resolve } from "node:path";

const bin = settings.path || which(["krita", "org.kde.krita"]);
const [cmd, ...rest] = args;
if (cmd === "status" || cmd === "connect") { if (!bin) fail("Krita not found (sudo apt install krita, or set its path)."); console.log(`Krita found: ${bin}`); process.exit(0); }
if (!bin) fail("Krita not found.");
switch (cmd) {
  case "export": passthrough(bin, [resolve(rest[0]), "--export", "--export-filename", resolve(rest[1])]); break;
  case "export-sequence": passthrough(bin, [resolve(rest[0]), "--export-sequence", "--export-filename", resolve(rest[1])]); break;
  case "open": passthrough(bin, rest.map((x) => resolve(x))); break;
  default: console.log(`Usage:
  export <file.kra> <out.png>                    export a document
  export-sequence <animation.kra> <frame.png>    export animation frames
  open <files...>                                open in Krita`);
}
