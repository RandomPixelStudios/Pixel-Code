// Amazon Web Services via the AWS CLI (downloaded on connect if missing) and an access key.
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { homedir } from "node:os";
import { args, settings, fail, which, passthrough } from "./common.mjs";

const local = join(homedir(), ".local/share/pixel-code/aws/bin/aws");
const bin = which(["aws"]) || (existsSync(local) ? local : null);
Object.assign(process.env, {
  AWS_ACCESS_KEY_ID: settings.access_key_id || "",
  AWS_SECRET_ACCESS_KEY: settings.secret_access_key || "",
  AWS_DEFAULT_REGION: settings.region || "eu-central-1",
  AWS_PAGER: "",
});
const [cmd, ...rest] = args;
if (cmd && cmd !== "help" && (!settings.access_key_id || !settings.secret_access_key)) fail("Access key id and secret are required.");

function install() {
  const dir = join(homedir(), ".local/share/pixel-code/aws");
  const arch = process.arch === "arm64" ? "aarch64" : "x86_64";
  const r = spawnSync("sh", ["-c", `set -e; t=$(mktemp -d); curl -fsSL https://awscli.amazonaws.com/awscli-exe-linux-${arch}.zip -o $t/a.zip; unzip -q $t/a.zip -d $t; $t/aws/install -i '${dir}' -b '${dir}/bin' --update; rm -rf $t`], { stdio: "ignore" });
  if (r.status !== 0) fail("Installing the AWS CLI failed (needs curl and unzip).");
}

switch (cmd) {
  case "connect": case "status": {
    if (!bin) { if (cmd === "status") fail("AWS CLI not installed - press Connect"); install(); }
    const r = spawnSync(bin || local, ["sts", "get-caller-identity", "--output", "json"], { encoding: "utf8" });
    if (r.status !== 0) fail((r.stderr || "AWS login failed").trim().split("\n").pop());
    console.log(`Connected as ${JSON.parse(r.stdout).Arn} (${process.env.AWS_DEFAULT_REGION})`);
    break;
  }
  case undefined: case "help": console.log(`Usage: any AWS CLI command, e.g.
  s3 ls | s3 sync dist s3://my-bucket --delete | ec2 describe-instances
  lambda update-function-code --function-name x --zip-file fileb://fn.zip | logs tail /aws/lambda/x --since 1h`); break;
  default: if (!bin) fail("AWS CLI not installed - press Connect."); passthrough(bin, args);
}
