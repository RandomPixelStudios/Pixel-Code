// Sentry: read errors (issues) and their stack traces.
import { args, settings, fail, http, print, rawCall, flags } from "./common.mjs";

const base = (settings.url || "https://sentry.io").replace(/\/$/, "") + "/api/0";
const headers = { Authorization: `Bearer ${settings.token || ""}` };
const get = (p) => http("GET", base + p, { headers });
const org = settings.org;
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && (!settings.token || !org)) fail("Sentry token and organization slug are required.");

switch (cmd) {
  case "connect": case "status": { const p = await get(`/organizations/${org}/projects/`); console.log(`Connected to ${org} (${p.length} projects)`); break; }
  case "projects": print((await get(`/organizations/${org}/projects/`)).map((p) => p.slug).join("\n")); break;
  case "issues": print((await get(`/organizations/${org}/issues/?query=${encodeURIComponent(f.query || "is:unresolved")}&limit=25${w[0] ? `&project=${w[0]}` : ""}`)).map((i) => `${i.id} [${i.level}] ${i.title} (${i.count}x, last ${i.lastSeen}) ${i.permalink}`).join("\n")); break;
  case "issue": {
    const e = await get(`/organizations/${org}/issues/${w[0]}/events/latest/`);
    const ex = e.entries?.find((x) => x.type === "exception")?.data?.values || [];
    console.log(`${e.title}\n${e.culprit || ""}\n`);
    for (const v of ex) {
      console.log(`${v.type}: ${v.value}`);
      for (const fr of (v.stacktrace?.frames || []).slice(-15).reverse()) console.log(`  at ${fr.function || "?"} (${fr.filename}:${fr.lineNo})${fr.context?.length ? "\n     " + (fr.context.find((c) => c[0] === fr.lineNo)?.[1] || "").trim() : ""}`);
    }
    break;
  }
  case "resolve": print(await http("PUT", `${base}/organizations/${org}/issues/${w[0]}/`, { headers, body: { status: "resolved" } })); break;
  case "raw": await rawCall(base, headers, rest); break;
  default: console.log(`Usage:
  projects | issues [project] [--query "is:unresolved"] | issue <id> (stack trace) | resolve <id>
  raw <METHOD> </api/0 path> [json]`);
}
