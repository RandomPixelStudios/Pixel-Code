// SonarCloud / SonarQube: quality gate, issues and metrics.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const base = (settings.url || "https://sonarcloud.io").replace(/\/$/, "") + "/api";
await restPlugin({
  base, headers: { Authorization: `Bearer ${settings.token || ""}` }, need: [settings.token],
  status: async (req) => { const r = await req("GET", "/authentication/validate"); if (!r.valid) throw new Error("invalid token"); return "Token valid"; },
  commands: {
    gate: async (req, [project]) => { const r = (await req("GET", `/qualitygates/project_status?projectKey=${project}`)).projectStatus; return `Quality gate: ${r.status}\n` + r.conditions.map((c) => `  ${c.status}  ${c.metricKey} = ${c.actualValue} (${c.comparator} ${c.errorThreshold})`).join("\n"); },
    issues: async (req, [project, sev]) => (await req("GET", `/issues/search?componentKeys=${project}&resolved=false&ps=50${sev ? "&severities=" + sev : ""}`)).issues.map((i) => `${i.severity}  ${i.component.split(":").pop()}:${i.line ?? ""}  ${i.message}`).join("\n"),
    metrics: async (req, [project]) => (await req("GET", `/measures/component?component=${project}&metricKeys=bugs,vulnerabilities,code_smells,coverage,duplicated_lines_density,ncloc`)).component.measures.map((m) => `${m.metric}: ${m.value}`).join("\n"),
  },
  help: "Usage:\n  gate <project_key> | issues <project_key> [BLOCKER,CRITICAL] | metrics <project_key>",
});
