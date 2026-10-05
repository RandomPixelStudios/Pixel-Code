// GIMP in batch mode (Script-Fu / Python-Fu without the UI).
import { args, settings, fail, which, passthrough } from "./common.mjs";
import { resolve } from "node:path";

const bin = settings.path || which(["gimp-console-3.0", "gimp-console-2.10", "gimp-console", "gimp"]);
const [cmd, ...rest] = args;
if (cmd === "status" || cmd === "connect") { if (!bin) fail("GIMP not found (sudo apt install gimp, or set its path)."); console.log(`GIMP found: ${bin}`); process.exit(0); }
if (!bin) fail("GIMP not found.");
const q = (s) => JSON.stringify(resolve(s));

switch (cmd) {
  case "convert": passthrough(bin, ["-i", "-b", `(let* ((img (car (gimp-file-load RUN-NONINTERACTIVE ${q(rest[0])} ${q(rest[0])}))) (drw (car (gimp-image-flatten img)))) (gimp-file-save RUN-NONINTERACTIVE img drw ${q(rest[1])} ${q(rest[1])}))`, "-b", "(gimp-quit 0)"]); break;
  case "scale": passthrough(bin, ["-i", "-b", `(let* ((img (car (gimp-file-load RUN-NONINTERACTIVE ${q(rest[0])} ${q(rest[0])})))) (gimp-image-scale img ${rest[2]} ${rest[3]}) (let ((drw (car (gimp-image-flatten img)))) (gimp-file-save RUN-NONINTERACTIVE img drw ${q(rest[1])} ${q(rest[1])})))`, "-b", "(gimp-quit 0)"]); break;
  case "batch": passthrough(bin, ["-i", "-b", rest.join(" "), "-b", "(gimp-quit 0)"]); break;
  default: console.log(`Usage:
  convert <in> <out>                 convert/flatten an image (format from extension)
  scale <in> <out> <width> <height>  resize an image
  batch <script-fu expression>       run any Script-Fu code`);
}
