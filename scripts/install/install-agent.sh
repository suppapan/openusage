#!/usr/bin/env bash
# OpenUsage Agent installer for Linux and macOS.
#
# One-liner install (interactive prompts):
#   curl -fsSL https://github.com/suppapan/openusage/releases/latest/download/install-agent.sh | bash
#
# Non-interactive install:
#   curl -fsSL https://github.com/suppapan/openusage/releases/latest/download/install-agent.sh | bash -s -- \
#     --token YOUR_SYNC_TOKEN \
#     --relay https://relay.example.com:8090
#
# Options:
#   --token <TOKEN>        Sync token from the dashboard (prompted if omitted)
#   --relay <URL>          Relay server URL (prompted if omitted)
#   --machine-name <NAME>  Display name (default: hostname)
#   --interval <SECONDS>   Push interval (default: 300)
#   --install-dir <PATH>   Install directory (default: /usr/local/bin)
#   --no-service           Skip installing as a background service

set -euo pipefail

TOKEN=""
RELAY=""
MACHINE_NAME=""
INTERVAL="300"
INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
INSTALL_SERVICE="yes"
REPO="suppapan/openusage"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --token) TOKEN="$2"; shift 2 ;;
    --relay) RELAY="$2"; shift 2 ;;
    --machine-name) MACHINE_NAME="$2"; shift 2 ;;
    --interval) INTERVAL="$2"; shift 2 ;;
    --install-dir) INSTALL_DIR="$2"; shift 2 ;;
    --no-service) INSTALL_SERVICE="no"; shift ;;
    -h|--help)
      sed -n '2,20p' "$0"
      exit 0
      ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

# ─── Detect OS and arch ─────────────────────────────────────────────────────
OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)  PLATFORM="linux" ;;
  Darwin) PLATFORM="macos" ;;
  *) echo "Unsupported OS: $OS"; exit 1 ;;
esac

case "$ARCH" in
  x86_64|amd64)  ARCH_TAG="x86_64" ;;
  arm64|aarch64)
    # Use 'arm64' suffix on macOS, 'aarch64' on Linux (matches release naming)
    if [[ "$PLATFORM" == "macos" ]]; then ARCH_TAG="arm64"; else ARCH_TAG="aarch64"; fi
    ;;
  *) echo "Unsupported arch: $ARCH"; exit 1 ;;
esac

BINARY_NAME="openusage-agent-${PLATFORM}-${ARCH_TAG}"

echo ">>> OpenUsage Agent installer"
echo "    Platform: $PLATFORM-$ARCH_TAG"
echo "    Binary:   $BINARY_NAME"

# ─── Resolve latest release (includes prereleases) ──────────────────────────
TAG="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases?per_page=1" \
  | grep -E '"tag_name"' | head -1 | sed 's/.*"tag_name": *"\(.*\)".*/\1/')"
if [[ -z "$TAG" ]]; then
  echo "Failed to resolve latest release tag"; exit 1
fi
echo "    Release:  $TAG"

DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${TAG}/${BINARY_NAME}"

# ─── Prompt for token / relay if missing ────────────────────────────────────
if [[ -z "$TOKEN" ]]; then
  read -r -p "Sync token: " TOKEN
fi
if [[ -z "$RELAY" ]]; then
  read -r -p "Relay URL (e.g. https://relay.example.com:8090): " RELAY
fi
if [[ -z "$TOKEN" || -z "$RELAY" ]]; then
  echo "Token and relay URL are required"; exit 1
fi

MACHINE_NAME_ARG=""
if [[ -n "$MACHINE_NAME" ]]; then
  MACHINE_NAME_ARG="--machine-name '${MACHINE_NAME}'"
fi

# ─── Download binary ────────────────────────────────────────────────────────
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
TMP_BIN="$TMP_DIR/openusage-agent"

echo ">>> Downloading from $DOWNLOAD_URL"
curl -fsSL -o "$TMP_BIN" "$DOWNLOAD_URL"
chmod +x "$TMP_BIN"

# ─── Install binary ─────────────────────────────────────────────────────────
TARGET_BIN="${INSTALL_DIR}/openusage-agent"
if [[ -w "$INSTALL_DIR" ]]; then
  mv "$TMP_BIN" "$TARGET_BIN"
else
  echo ">>> Using sudo to install to $INSTALL_DIR"
  sudo mv "$TMP_BIN" "$TARGET_BIN"
fi
echo ">>> Installed to $TARGET_BIN"

# ─── Optional: set up service ──────────────────────────────────────────────
if [[ "$INSTALL_SERVICE" != "yes" ]]; then
  echo ""
  echo ">>> Done. Run manually with:"
  echo "    $TARGET_BIN --token $TOKEN --relay $RELAY"
  exit 0
fi

if [[ "$PLATFORM" == "linux" ]]; then
  SERVICE_PATH="/etc/systemd/system/openusage-agent.service"
  echo ">>> Installing systemd service at $SERVICE_PATH"
  sudo bash -c "cat > $SERVICE_PATH" <<EOF
[Unit]
Description=OpenUsage Agent
After=network.target

[Service]
Type=simple
ExecStart=${TARGET_BIN} --token ${TOKEN} --relay ${RELAY} --interval ${INTERVAL} ${MACHINE_NAME_ARG}
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF
  sudo systemctl daemon-reload
  sudo systemctl enable --now openusage-agent
  echo ">>> Service running. Check status:"
  echo "    sudo systemctl status openusage-agent"
  echo ">>> Tail logs:"
  echo "    sudo journalctl -u openusage-agent -f"
elif [[ "$PLATFORM" == "macos" ]]; then
  PLIST_PATH="$HOME/Library/LaunchAgents/com.openusage.agent.plist"
  mkdir -p "$(dirname "$PLIST_PATH")"
  echo ">>> Installing launchd agent at $PLIST_PATH"
  cat > "$PLIST_PATH" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.openusage.agent</string>
  <key>ProgramArguments</key>
  <array>
    <string>${TARGET_BIN}</string>
    <string>--token</string>
    <string>${TOKEN}</string>
    <string>--relay</string>
    <string>${RELAY}</string>
    <string>--interval</string>
    <string>${INTERVAL}</string>
EOF
  if [[ -n "$MACHINE_NAME" ]]; then
    cat >> "$PLIST_PATH" <<EOF
    <string>--machine-name</string>
    <string>${MACHINE_NAME}</string>
EOF
  fi
  cat >> "$PLIST_PATH" <<EOF
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/tmp/openusage-agent.log</string>
  <key>StandardErrorPath</key>
  <string>/tmp/openusage-agent.err.log</string>
</dict>
</plist>
EOF
  launchctl unload "$PLIST_PATH" 2>/dev/null || true
  launchctl load "$PLIST_PATH"
  echo ">>> Agent running via launchd."
  echo ">>> Tail logs:"
  echo "    tail -f /tmp/openusage-agent.log"
fi

echo ""
echo ">>> Setup complete. Agent will push to $RELAY every ${INTERVAL}s."
