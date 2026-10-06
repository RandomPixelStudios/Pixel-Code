// Codecov: coverage of your repositories.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const svc = settings.service || "github";
await restPlugin({
  base: `https://api.codecov.io/api/v2/${svc}`, headers: { Authorization: `Bearer ${settings.token || ""}` }, need: [settings.token, settings.owner],
  status: async (req) => `Connected (${(await req("GET", `/${settings.owner}/repos/?page_size=100`)).count} repos)`,
  commands: {
    repos: async (req) => (await req("GET", `/${settings.owner}/repos/?page_size=100`)).results.map((r) => `${r.name}  ${r.totals?.coverage ?? "-"}%`).join("\n"),
    coverage: async (req, [repo, branch]) => { const r = await req("GET", `/${settings.owner}/repos/${repo}/totals/${branch ? "?branch=" + branch : ""}`); return `${r.coverage}% (${r.hits}/${r.lines} lines)`; },
    files: async (req, [repo]) => (await req("GET", `/${settings.owner}/repos/${repo}/report/`)).files.sort((a, b) => a.totals.coverage - b.totals.coverage).slice(0, 30).map((f) => `${f.totals.coverage.toFixed(1).padStart(5)}%  ${f.name}`).join("\n"),
  },
  help: "Usage:\n  repos | coverage <repo> [branch] | files <repo>   (least covered files first)",
});
