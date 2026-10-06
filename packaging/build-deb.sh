#!/bin/sh
# Baut das .deb für Ubuntu, Debian und Linux Mint.
# Kompiliert in Ubuntu 22.04 (glibc 2.35): läuft auf Ubuntu 22.04+, Linux Mint 21+ und Debian 12+.
# Aufruf: packaging/build-deb.sh   ->  dist/pixel-code_<version>_amd64.deb
set -eu
cd "$(dirname "$0")/.."
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
ARCH=amd64

# Rust und Cargo-Cache liegen in Docker-Volumes, damit spätere Builds schnell sind.
docker run --rm -v "$PWD":/src -w /src \
  -v pixel-code-cargo:/usr/local/cargo -v pixel-code-rustup:/usr/local/rustup \
  -e CARGO_HOME=/usr/local/cargo -e RUSTUP_HOME=/usr/local/rustup -e CARGO_TARGET_DIR=/src/target/deb \
  ubuntu:22.04 sh -c "
    set -e
    export DEBIAN_FRONTEND=noninteractive; apt-get update -qq && apt-get install -y -qq curl ca-certificates gcc libc6-dev pkg-config >/dev/null
    [ -x /usr/local/cargo/bin/cargo ] || curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal --no-modify-path
    /usr/local/cargo/bin/rustup update stable --no-self-update >/dev/null 2>&1 || true
    /usr/local/cargo/bin/cargo build --release --locked
    chown -R $(id -u):$(id -g) /src/target/deb
  "

PKG=target/deb/pkg
rm -rf "$PKG"
install -Dm755 target/deb/release/pixel-code "$PKG/usr/bin/pixel-code"
install -Dm644 packaging/pixel-code.desktop "$PKG/usr/share/applications/pixel-code.desktop"
install -Dm644 assets/icon.svg "$PKG/usr/share/icons/hicolor/scalable/apps/pixel-code.svg"
strip "$PKG/usr/bin/pixel-code" 2>/dev/null || true
SIZE=$(du -sk "$PKG/usr" | cut -f1)
mkdir -p "$PKG/DEBIAN"
cat > "$PKG/DEBIAN/control" <<CONTROL
Package: pixel-code
Version: $VERSION
Section: devel
Priority: optional
Architecture: $ARCH
Installed-Size: $SIZE
Depends: libc6 (>= 2.35), libgcc-s1, libgl1, libxkbcommon0, curl
Recommends: nodejs, git, policykit-1 | pkexec, xdg-desktop-portal
Maintainer: RandomPixelStudios <randompixxelstudios@gmail.com>
Homepage: https://github.com/RandomPixelStudios/Pixel-Code
Description: Terminal workspace for AI coding agents
 Pixel Code runs Claude Code, Codex, OpenCode, Gemini CLI and other coding
 agents side by side, shows what each one is doing (working, question,
 permission, done) and connects them to plugins like GitHub, ioBroker,
 Home Assistant, Docker and many more.
CONTROL
cat > "$PKG/DEBIAN/postinst" <<'POST'
#!/bin/sh
set -e
command -v update-desktop-database >/dev/null && update-desktop-database -q /usr/share/applications || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t /usr/share/icons/hicolor || true
POST
chmod 755 "$PKG/DEBIAN/postinst"
mkdir -p dist
OUT="dist/pixel-code_${VERSION}_${ARCH}.deb"
dpkg-deb --root-owner-group --build "$PKG" "$OUT"
echo "$OUT"
