// Weights & Biases: training runs and metrics.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const gql = async (query) => { const r = await http("POST", "https://api.wandb.ai/graphql", { headers: { Authorization: "Basic " + Buffer.from("api:" + (settings.api_key || "")).toString("base64") }, body: { query } }); if (r.errors) fail(r.errors[0].message); return r.data; };
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.api_key) fail("No W&B API key set.");
switch (cmd) {
  case "connect": case "status": { const d = await gql("{ viewer { username entity } }"); console.log("Connected as " + d.viewer.username); break; }
  case "projects": { const d = await gql(`{ viewer { entity } }`); print((await gql(`{ projects(entityName: "${rest[0] || d.viewer.entity}", first: 50) { edges { node { name runCount } } } }`)).projects.edges.map((e) => `${e.node.name}  ${e.node.runCount} runs`).join("\n")); break; }
  case "runs": { const [entity, project] = rest[0].split("/"); print((await gql(`{ project(name: "${project}", entityName: "${entity}") { runs(first: 20, order: "-created_at") { edges { node { name displayName state summaryMetrics } } } } }`)).project.runs.edges.map((e) => `${e.node.name}  ${e.node.displayName}  ${e.node.state}  ${e.node.summaryMetrics?.slice(0, 200)}`).join("\n")); break; }
  default: console.log("Usage:\n  projects [entity] | runs <entity/project>");
}
