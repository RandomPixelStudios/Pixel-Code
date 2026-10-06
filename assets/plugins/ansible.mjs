// Ansible - generated plugin, see Pixel Code > Settings > Plugins.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";

cliPlugin({ bins: ["ansible-playbook"], name: "Ansible", version: ["--version"], prefix: [], env: { ANSIBLE_INVENTORY: settings.inventory }, help: `Usage: arguments for ansible-playbook, e.g. 'site.yml -i inventory --check'` });
