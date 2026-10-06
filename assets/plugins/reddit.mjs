// Reddit (script app): post devlogs and read subreddits.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const UA = "PixelCode/0.3 by " + (settings.username || "user");
async function token() {
  const r = await http("POST", "https://www.reddit.com/api/v1/access_token", { headers: { Authorization: "Basic " + Buffer.from(`${settings.client_id}:${settings.client_secret}`).toString("base64"), "User-Agent": UA }, form: { grant_type: "password", username: settings.username, password: settings.password } });
  if (!r.access_token) fail("Reddit login failed: " + JSON.stringify(r));
  return { Authorization: `Bearer ${r.access_token}`, "User-Agent": UA };
}
const api = async (m, p, form) => http(m, "https://oauth.reddit.com" + p, { headers: await token(), ...(form ? { form } : {}) });
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && !(settings.client_id && settings.client_secret && settings.username && settings.password)) fail("Client id, secret, username and password are required.");
switch (cmd) {
  case "connect": case "status": { const me = await api("GET", "/api/v1/me"); console.log(`Connected as u/${me.name} (${me.total_karma} karma)`); break; }
  case "post": { const r = await api("POST", "/api/submit", { sr: w[0], kind: f.url ? "link" : "self", title: f.title || w.slice(1).join(" "), ...(f.url ? { url: f.url } : { text: f.text || "" }), ...(f.flair ? { flair_id: f.flair } : {}), api_type: "json" }); print(r.json?.errors?.length ? r.json.errors : r.json?.data?.url); break; }
  case "read": print((await api("GET", `/r/${w[0]}/${f.sort || "hot"}?limit=${f.n || 15}`)).data.children.map(({ data: p }) => `${p.score}  ${p.title}  (${p.num_comments} comments) https://reddit.com${p.permalink}`).join("\n")); break;
  case "comments": print((await api("GET", `/comments/${w[0]}?limit=30`))[1].data.children.filter((c) => c.kind === "t1").map(({ data: c }) => `${c.author}: ${c.body}`).join("\n\n")); break;
  case "reply": { await api("POST", "/api/comment", { thing_id: w[0], text: w.slice(1).join(" "), api_type: "json" }); console.log("Replied."); break; }
  default: console.log(`Usage:
  post <subreddit> --title "..." [--text "markdown"] [--url link] [--flair id]
  read <subreddit> [--sort hot|new|top] [--n 15] | comments <post_id> | reply <t1_xxx|t3_xxx> <text>`);
}
