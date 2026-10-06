// Kaggle: datasets, competitions and notebooks.
import { spawnSync } from "node:child_process";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const env = { ...process.env, KAGGLE_USERNAME: settings.username || "", KAGGLE_KEY: settings.key || "" };
const bin = which(["kaggle"]);
const run = (a) => process.exit((bin ? spawnSync(bin, a, { stdio: "inherit", env }) : spawnSync(which(["uvx"]) || "uvx", ["kaggle", ...a], { stdio: "inherit", env })).status ?? 1);
const [cmd] = args;
if (cmd === "status" || cmd === "connect") { if (!settings.username || !settings.key) fail("Username and API key are required."); const r = bin ? spawnSync(bin, ["config", "view"], { env, encoding: "utf8" }) : spawnSync("uvx", ["kaggle", "config", "view"], { env, encoding: "utf8" }); if (r.status !== 0) fail("Kaggle CLI not available (pip install kaggle or install uv)."); console.log("Kaggle ready for " + settings.username); process.exit(0); }
if (!cmd || cmd === "help") { console.log("Usage: any kaggle arguments, e.g. 'datasets list -s minecraft', 'datasets download -d owner/name --unzip', 'competitions list', 'kernels push'"); process.exit(0); }
run(args);
