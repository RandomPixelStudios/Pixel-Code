// Docker on this machine.
import { spawnSync } from "node:child_process";
import { args, fail, passthrough } from "./common.mjs";

const [cmd, ...rest] = args;
if (cmd === "status" || cmd === "connect") {
  const r = spawnSync("docker", ["version", "--format", "{{.Server.Version}}"], { encoding: "utf8" });
  if (r.status !== 0) fail("Docker is not available (is it installed and is your user in the docker group?)");
  console.log(`Docker ${r.stdout.trim()} running`);
  process.exit(0);
}
switch (cmd) {
  case "ps": passthrough("docker", ["ps", "-a", "--format", "table {{.Names}}\t{{.Image}}\t{{.Status}}\t{{.Ports}}"]); break;
  case "logs": passthrough("docker", ["logs", "--tail", rest[1] || "200", rest[0]]); break;
  case undefined: case "help": console.log(`Usage:
  ps                      list containers
  logs <name> [lines]     last log lines
  <any docker arguments>  e.g. 'compose up -d', 'build -t app .', 'exec web ls'`); break;
  default: passthrough("docker", args);
}
