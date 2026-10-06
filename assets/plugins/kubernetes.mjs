// Kubernetes through kubectl.
import { args, settings, fail, passthrough, which } from "./common.mjs";
import { spawnSync } from "node:child_process";

const kubectl = which(["kubectl"]);
const base = [...(settings.context ? ["--context", settings.context] : []), ...(settings.kubeconfig ? ["--kubeconfig", settings.kubeconfig.replace(/^~/, process.env.HOME)] : [])];
const [cmd] = args;
if (cmd === "status" || cmd === "connect") {
  if (!kubectl) fail("kubectl is not installed.");
  const r = spawnSync(kubectl, [...base, "config", "current-context"], { encoding: "utf8" });
  if (r.status !== 0) fail("No kubectl context configured.");
  console.log(`Context ${r.stdout.trim()}`);
  process.exit(0);
}
if (!cmd || cmd === "help") {
  console.log(`Usage: any kubectl arguments, e.g. 'get pods -A', 'logs deploy/web', 'apply -f k8s/', 'rollout restart deploy/web'`);
  process.exit(0);
}
if (!kubectl) fail("kubectl is not installed.");
passthrough(kubectl, [...base, ...args]);
