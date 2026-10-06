// Wake-on-LAN: switch on PCs and servers in your network.
import { createSocket } from "node:dgram";
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
const devices = Object.fromEntries((settings.devices || "").split(",").map((x) => x.trim().split("=").map((s) => s.trim())).filter((x) => x.length === 2));
function wake(mac) {
  const hex = mac.replace(/[^0-9a-f]/gi, "");
  if (hex.length !== 12) fail("Invalid MAC address: " + mac);
  const m = Buffer.from(hex, "hex"), pkt = Buffer.concat([Buffer.alloc(6, 0xff), ...Array(16).fill(m)]);
  return new Promise((ok) => { const s = createSocket("udp4"); s.bind(() => { s.setBroadcast(true); s.send(pkt, 9, settings.broadcast || "255.255.255.255", () => { s.close(); ok(); }); }); });
}
const [cmd, ...rest] = args;
switch (cmd) {
  case "connect": case "status": if (!Object.keys(devices).length) fail("Add devices as name=MAC, e.g. server=AA:BB:CC:DD:EE:FF"); console.log("Devices: " + Object.keys(devices).join(", ")); break;
  case "list": print(Object.entries(devices).map(([n, m]) => `${n}  ${m}`).join("\n")); break;
  case "wake": await wake(devices[rest[0]] || rest[0]); console.log("Magic packet sent to " + rest[0]); break;
  default: console.log("Usage:\n  list | wake <name|MAC>");
}
