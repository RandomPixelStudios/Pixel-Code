// Jupyter notebooks: execute and convert.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const [cmd, ...rest] = args;
const bin = which(["jupyter"]);
switch (cmd) {
  case "connect": case "status": if (!bin) fail("jupyter not found (pip install jupyter or uvx)."); console.log("Jupyter ready"); break;
  case "run": passthrough(bin, ["nbconvert", "--to", "notebook", "--execute", "--inplace", ...rest]); break;
  case "convert": passthrough(bin, ["nbconvert", "--to", rest[1] || "markdown", rest[0]]); break;
  default: if (cmd && bin) passthrough(bin, args); else console.log("Usage:\n  run <notebook.ipynb> (executes and saves outputs) | convert <notebook.ipynb> [markdown|html|script|pdf] | any jupyter arguments");
}
