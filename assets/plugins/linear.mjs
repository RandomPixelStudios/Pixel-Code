// Linear issues via GraphQL API key.
import { args, settings, fail, http, print } from "./common.mjs";

const gql = async (query, variables) => {
  const r = await http("POST", "https://api.linear.app/graphql", { headers: { Authorization: settings.api_key || "" }, body: { query, variables } });
  if (r.errors) fail(`Linear: ${r.errors[0].message}`);
  return r.data;
};
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !settings.api_key) fail("No Linear API key set.");

switch (cmd) {
  case "connect": case "status": { const d = await gql("{ viewer { name } organization { name } }"); console.log(`Connected as ${d.viewer.name} (${d.organization.name})`); break; }
  case "teams": print((await gql("{ teams { nodes { id key name } } }")).teams.nodes.map((t) => `${t.key} ${t.id} ${t.name}`).join("\n")); break;
  case "issues": print((await gql(`{ issues(first: 40, filter: { state: { type: { nin: ["completed", "canceled"] } } ${rest[0] ? `, team: { key: { eq: "${rest[0]}" } }` : ""} }) { nodes { identifier title state { name } assignee { name } url } } }`)).issues.nodes.map((i) => `${i.identifier} [${i.state.name}] ${i.title} ${i.assignee ? "@" + i.assignee.name : ""}`).join("\n")); break;
  case "issue": { const i = (await gql(`query($id:String!){ issue(id:$id){ identifier title description state{name} url comments{nodes{body user{name}}} } }`, { id: rest[0] })).issue; print(`${i.identifier} ${i.title} [${i.state.name}]\n${i.url}\n\n${i.description ?? ""}\n\n${i.comments.nodes.map((c) => `${c.user?.name}: ${c.body}`).join("\n")}`); break; }
  case "create": { const teams = (await gql("{ teams { nodes { id key } } }")).teams.nodes; const team = teams.find((t) => t.key === rest[0]); if (!team) fail(`Unknown team key ${rest[0]}`); const r = await gql("mutation($i:IssueCreateInput!){ issueCreate(input:$i){ issue{ identifier url } } }", { i: { teamId: team.id, title: rest[1], description: rest.slice(2).join(" ") } }); print(`${r.issueCreate.issue.identifier} ${r.issueCreate.issue.url}`); break; }
  case "comment": { const id = (await gql(`query($id:String!){ issue(id:$id){ id } }`, { id: rest[0] })).issue.id; await gql("mutation($i:CommentCreateInput!){ commentCreate(input:$i){ success } }", { i: { issueId: id, body: rest.slice(1).join(" ") } }); console.log("Commented."); break; }
  case "state": { const iss = (await gql(`query($id:String!){ issue(id:$id){ id team { states { nodes { id name } } } } }`, { id: rest[0] })).issue; const st = iss.team.states.nodes.find((s) => s.name.toLowerCase() === rest.slice(1).join(" ").toLowerCase()); if (!st) fail(`States: ${iss.team.states.nodes.map((s) => s.name).join(", ")}`); await gql("mutation($id:String!,$s:String!){ issueUpdate(id:$id, input:{stateId:$s}){ success } }", { id: iss.id, s: st.id }); console.log("Updated."); break; }
  case "gql": print(await gql(rest.join(" "))); break;
  default: console.log(`Usage:
  teams | issues [TEAM] | issue <ABC-12> | create <TEAM> <title> [description] | comment <ABC-12> <text>
  state <ABC-12> <state name> | gql <graphql query>`);
}
