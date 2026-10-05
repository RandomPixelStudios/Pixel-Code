// Run commands on your SSH servers (uses your normal SSH keys / ~/.ssh/config).
import { spawnSync } from "node:child_process";
import { args, settings, fail } from "./common.mjs";

const hosts = (settings.hosts || "").split(/[\s,]+/).filter(Boolean);
const [cmd, ...rest] = args;
const ssh = (host, a, opts = {}) => spawnSync("ssh", ["-o", "BatchMode=yes", "-o", "ConnectTimeout=8", host, ...a], { encoding: "utf8", ...opts });

switch (cmd) {
  case "connect": case "status": {
    if (!hosts.length) fail("Add at least one host (user@server or a Host from ~/.ssh/config).");
    const bad = hosts.filter((h) => ssh(h, ["true"]).status !== 0);
    if (bad.length) fail(`Cannot log in without password to: ${bad.join(", ")} (set up SSH keys with ssh-copy-id)`);
    console.log(`${hosts.length} host(s) reachable: ${hosts.join(", ")}`);
    break;
  }
  case "hosts": console.log(hosts.join("\n")); break;
  case "run": {
    const [host, ...c] = rest;
    if (!hosts.includes(host)) fail(`Unknown host "${host}". Allowed: ${hosts.join(", ")}`);
    const r = ssh(host, [c.join(" ")], { stdio: "inherit" });
    process.exit(r.status ?? 1);
  }
  case "copy": {
    const r = spawnSync("scp", ["-o", "BatchMode=yes", "-r", rest[0], rest[1]], { stdio: "inherit" });
    process.exit(r.status ?? 1);
  }
  default: console.log(`Usage:
  hosts                       configured servers
  run <host> <command>        run a command on a server
  copy <from> <to>            scp, e.g. copy dist/ server:/var/www/`);
}
