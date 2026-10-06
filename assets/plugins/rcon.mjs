// Minecraft server console over RCON (enable-rcon=true in server.properties).
import { connect } from "node:net";
import { readFileSync, statSync, openSync, readSync, closeSync } from "node:fs";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const host = settings.host || "127.0.0.1", port = Number(settings.port || 25575);
function rcon(commands) {
  return new Promise((ok, bad) => {
    const sock = connect({ host, port });
    const pkt = (rid, type, body) => { const b = Buffer.from(body, "utf8"); const p = Buffer.alloc(14 + b.length); p.writeInt32LE(10 + b.length, 0); p.writeInt32LE(rid, 4); p.writeInt32LE(type, 8); b.copy(p, 12); return p; };
    const out = new Map();
    let buf = Buffer.alloc(0), authed = false;
    const timer = setTimeout(() => { sock.destroy(); bad(new Error("RCON timeout")); }, 15000);
    sock.on("connect", () => sock.write(pkt(1, 3, settings.password || "")));
    sock.on("data", (d) => {
      buf = Buffer.concat([buf, d]);
      while (buf.length >= 4 && buf.length >= buf.readInt32LE(0) + 4) {
        const len = buf.readInt32LE(0), rid = buf.readInt32LE(4), body = buf.subarray(12, 4 + len - 2).toString("utf8");
        buf = buf.subarray(4 + len);
        if (!authed) {
          if (rid === -1) { clearTimeout(timer); sock.destroy(); return bad(new Error("Wrong RCON password")); }
          authed = true;
          commands.forEach((c, i) => sock.write(pkt(100 + i, 2, c)));
        } else {
          out.set(rid, (out.get(rid) || "") + body.replace(/\u00a7./g, ""));
          if (out.size >= commands.length) { clearTimeout(timer); sock.end(); ok(commands.map((_, i) => out.get(100 + i) || "")); }
        }
      }
    });
    sock.on("error", (e) => { clearTimeout(timer); bad(e); });
  });
}
const tail = (file, n) => { try { const s = statSync(file).size, fd = openSync(file, "r"), len = Math.min(s, 200000), b = Buffer.alloc(len); readSync(fd, b, 0, len, s - len); closeSync(fd); return b.toString("utf8").split("\n").slice(-n).join("\n"); } catch (e) { fail("Cannot read log: " + e.message); } };
const [cmd, ...rest] = args;
try {
  switch (cmd) {
    case "connect": case "status": { const [r] = await rcon(["list"]); console.log("Connected: " + r.trim()); break; }
    case "cmd": console.log((await rcon([rest.join(" ")])).join("\n") || "(no output)"); break;
    case "log": if (!settings.server_dir) fail("Set the server folder to read logs."); console.log(tail(settings.server_dir.replace(/\/$/, "") + "/logs/latest.log", Number(rest[0] || 80))); break;
    default: console.log(`Usage:
  cmd <command>     run a server command, e.g. 'cmd reload', 'cmd give @a diamond 1', 'cmd say hi'
  log [lines]       end of logs/latest.log (needs the server folder)`);
  }
} catch (e) { fail(`RCON ${host}:${port}: ${e.message}`); }
