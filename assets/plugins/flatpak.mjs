// Flatpak / Flathub: build and test Flatpak packages.
import { args, settings, fail, http, print, flags, which, passthrough, cliPlugin, restPlugin } from "./common.mjs";
cliPlugin({ bins: ["flatpak-builder"], name: "flatpak-builder", help: `Usage: any flatpak-builder arguments, e.g.
  '--user --install --force-clean build-dir com.example.App.yml'
  '--repo=repo --force-clean build-dir com.example.App.yml' (then submit the manifest as a PR to github.com/flathub/flathub)` });
