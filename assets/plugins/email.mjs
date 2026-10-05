// E-mail via SMTP (curl). Works with Gmail/Outlook app passwords and any SMTP server.
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { basename, resolve } from "node:path";
import { randomBytes } from "node:crypto";
import { args, settings, fail, flags } from "./common.mjs";

const s = settings;
const [cmd, ...rest] = args;
const [f, w] = flags(rest);
if (cmd && cmd !== "help" && (!s.smtp || !s.user || !s.password)) fail("SMTP server, user and password are required.");
const url = s.smtp.includes("://") ? s.smtp : `smtps://${s.smtp}${s.smtp.includes(":") ? "" : ":465"}`;
const from = s.from || s.user;

function send(to, subject, body, files = []) {
  const b = randomBytes(8).toString("hex");
  let msg = `From: ${from}\r\nTo: ${to}\r\nSubject: =?UTF-8?B?${Buffer.from(subject).toString("base64")}?=\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=${b}\r\n\r\n--${b}\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n${body}\r\n`;
  for (const file of files) msg += `--${b}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: attachment; filename="${basename(file)}"\r\nContent-Transfer-Encoding: base64\r\n\r\n${readFileSync(resolve(file)).toString("base64").replace(/.{76}/g, "$&\r\n")}\r\n`;
  msg += `--${b}--\r\n`;
  const rcpt = to.split(",").flatMap((t) => ["--mail-rcpt", t.trim()]);
  const r = spawnSync("curl", ["-s", "--ssl-reqd", "--url", url, "--user", `${s.user}:${s.password}`, "--mail-from", from, ...rcpt, "-T", "-"], { input: msg, encoding: "utf8" });
  if (r.status !== 0) fail(`SMTP failed (curl exit ${r.status}) ${r.stderr}`);
}

switch (cmd) {
  case "connect": case "status": {
    const r = spawnSync("curl", ["-s", "--ssl-reqd", "--url", url, "--user", `${s.user}:${s.password}`, "-X", "NOOP"], { encoding: "utf8" });
    if (r.status !== 0) fail(`SMTP login failed (curl exit ${r.status})`);
    console.log(`SMTP ok as ${s.user}`);
    break;
  }
  case "send": send(f.to || s.to || s.user, f.subject || "Message from your agent", w.join(" "), (f.attach ? String(f.attach).split(",") : [])); console.log("Sent."); break;
  default: console.log(`Usage:
  send <text> [--to a@b.c] [--subject "..."] [--attach file1,file2]   (default recipient: your own address)`);
}
