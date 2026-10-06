// MQTT - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

const [cmd, ...rest] = args;
const base = ["-h", settings.host || "localhost", ...(settings.user ? ["-u", settings.user, "-P", settings.password || ""] : [])];
if (cmd === "status" || cmd === "connect") {
  const pub = which(["mosquitto_pub"]);
  if (!pub) fail("mosquitto-clients is not installed (sudo apt install mosquitto-clients).");
  console.log(`MQTT broker ${settings.host || "localhost"}`);
} else if (cmd === "pub") passthrough(which(["mosquitto_pub"]) || "mosquitto_pub", [...base, ...rest]);
else if (cmd === "sub") passthrough(which(["mosquitto_sub"]) || "mosquitto_sub", [...base, "-v", ...rest]);
else console.log("Usage:\n  pub -t <topic> -m <message>\n  sub -t <topic> [-C count]");
