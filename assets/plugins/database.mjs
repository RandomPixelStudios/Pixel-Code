// SQL databases: PostgreSQL (psql), MySQL/MariaDB (mysql) and SQLite (sqlite3). Read-only unless allowed.
import { spawnSync } from "node:child_process";
import { args, settings, fail, which } from "./common.mjs";

const url = settings.url || "";
const writable = settings.allow_writes === "yes";
const kind = url.startsWith("postgres") ? "postgres" : url.startsWith("mysql") || url.startsWith("mariadb") ? "mysql" : url ? "sqlite" : null;
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && !kind) fail("No database URL set.");

function run(sql) {
  const ro = !writable;
  if (ro && /^\s*(insert|update|delete|drop|alter|create|truncate|grant|revoke)\b/i.test(sql)) fail("Writes are disabled. Allow them in the plugin settings (allow_writes = yes).");
  let r;
  if (kind === "postgres") {
    if (!which(["psql"])) fail("psql is not installed (sudo apt install postgresql-client).");
    r = spawnSync("psql", [url, "-X", "-v", "ON_ERROR_STOP=1", "-c", ro ? `BEGIN READ ONLY; ${sql}; COMMIT;` : sql], { encoding: "utf8" });
  } else if (kind === "mysql") {
    if (!which(["mysql", "mariadb"])) fail("mysql client is not installed (sudo apt install mariadb-client).");
    const u = new URL(url.replace(/^mariadb/, "mysql"));
    r = spawnSync(which(["mysql", "mariadb"]), ["-h", u.hostname, "-P", u.port || "3306", "-u", decodeURIComponent(u.username), `-p${decodeURIComponent(u.password)}`, u.pathname.slice(1), "-t", "-e", sql], { encoding: "utf8" });
  } else {
    if (!which(["sqlite3"])) fail("sqlite3 is not installed (sudo apt install sqlite3).");
    r = spawnSync("sqlite3", [...(ro ? ["-readonly"] : []), "-header", "-column", url.replace(/^sqlite:\/\//, ""), sql], { encoding: "utf8" });
  }
  if (r.status !== 0) fail((r.stderr || r.error?.message || "query failed").trim());
  console.log(r.stdout.trim());
}

switch (cmd) {
  case "connect": case "status": run("SELECT 1"); console.log(`Connected (${kind}${writable ? ", writes allowed" : ", read-only"})`); break;
  case "query": run(rest.join(" ")); break;
  case "tables": run(kind === "postgres" ? "SELECT table_schema, table_name FROM information_schema.tables WHERE table_schema NOT IN ('pg_catalog','information_schema') ORDER BY 1,2" : kind === "mysql" ? "SHOW TABLES" : "SELECT name FROM sqlite_master WHERE type='table'"); break;
  case "schema": run(kind === "postgres" ? `SELECT column_name, data_type, is_nullable FROM information_schema.columns WHERE table_name='${rest[0]}' ORDER BY ordinal_position` : kind === "mysql" ? `DESCRIBE ${rest[0]}` : `PRAGMA table_info(${rest[0]})`); break;
  default: console.log(`Usage:
  tables                 list tables
  schema <table>         columns of a table
  query <sql>            run SQL (read-only unless writes are allowed)`);
}
