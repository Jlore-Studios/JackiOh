#!/bin/sh
# `sh scripts/browse.sh <command> [args]`: a headless browser an agent drives one shell command at a
# time to look at the web client, for UI work only (.claude/skills/ui-walkthrough/SKILL.md, #811).
# Every argument goes to Microsoft's Playwright CLI (`@playwright/cli`): `open <url>`, `click`,
# `eval`, `resize`, `screenshot`, `close`, … (`sh scripts/browse.sh --help` lists them). The
# browser stays open between commands, until `close`.
#
# It costs nothing to anyone who never runs it, which is the point:
#   - the CLI is installed on first use into .cache/playwright-cli/ (gitignored, as build-wasm.sh
#     keeps wasm-bindgen in .cache/bin/) with npm, outside the pnpm workspace, so no package.json
#     or lockfile names it and no `pnpm install` (CI's, the night bot's, yours) fetches it. A run
#     that finds the pinned version there installs nothing;
#   - no browser is downloaded unless asked for (`sh scripts/browse.sh install-browser chromium`).
#     The browser is, in order: JACKIOH_BROWSER (a path to a Chrome or Chromium executable); the
#     Chromium a Claude Code cloud container ships at $PLAYWRIGHT_BROWSERS_PATH/chromium; a Chrome
#     or Chromium on PATH, or macOS's Google Chrome; else Playwright's own, which install-browser
#     downloads once.
#
# The CLI runs from .cache/browse/ (gitignored), where it reads the config this script writes
# (.playwright/cli.config.json: headless, a VIEWPORT_WIDTH x VIEWPORT_HEIGHT page, and Chromium's
# sandbox off only when running as root, as a cloud container does, since Chromium refuses root with
# it on). So a relative path, `screenshot --filename=board.png` among them, lands in .cache/browse/,
# and the CLI's own snapshots and console logs in .cache/browse/out/: a walkthrough never leaves a
# file to commit.

set -eu

PLAYWRIGHT_CLI_VERSION="0.1.22"
# The page size a browser opens at: BUILD's desktop viewport. `resize <w> <h>` changes it after.
VIEWPORT_WIDTH=1280
VIEWPORT_HEIGHT=720

ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
INSTALL_DIR="$ROOT/.cache/playwright-cli"
CLI="$INSTALL_DIR/node_modules/.bin/playwright-cli"
MANIFEST="$INSTALL_DIR/node_modules/@playwright/cli/package.json"
WORK_DIR="$ROOT/.cache/browse"
CONFIG="$WORK_DIR/.playwright/cli.config.json"

for tool in node npm; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "browse: $tool is not on PATH. Install Node (.nvmrc pins the version)." >&2
    exit 1
  fi
done

# The CLI, pinned. Its own dependencies (playwright, playwright-core) are exact versions, so the
# pin fixes the whole install.
installed=$(node -p "require(process.argv[1]).version" "$MANIFEST" 2>/dev/null || true)
if [ "$installed" != "$PLAYWRIGHT_CLI_VERSION" ] || [ ! -x "$CLI" ]; then
  echo "browse: installing @playwright/cli@$PLAYWRIGHT_CLI_VERSION into .cache/playwright-cli/ (once)" >&2
  rm -rf "$INSTALL_DIR"
  mkdir -p "$INSTALL_DIR"
  PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm install --prefix "$INSTALL_DIR" --no-save --no-audit --no-fund \
    --loglevel=error "@playwright/cli@$PLAYWRIGHT_CLI_VERSION" >&2
fi

# The browser.
browser=${JACKIOH_BROWSER:-}
if [ -n "$browser" ] && ! { [ -f "$browser" ] && [ -x "$browser" ]; }; then
  echo "browse: JACKIOH_BROWSER is \"$browser\", which is not an executable" >&2
  exit 1
fi
if [ -z "$browser" ] && [ -n "${PLAYWRIGHT_BROWSERS_PATH:-}" ] && [ -f "$PLAYWRIGHT_BROWSERS_PATH/chromium" ] \
  && [ -x "$PLAYWRIGHT_BROWSERS_PATH/chromium" ]; then
  browser="$PLAYWRIGHT_BROWSERS_PATH/chromium"
fi
if [ -z "$browser" ]; then
  for name in google-chrome-stable google-chrome chromium chromium-browser; do
    if found=$(command -v "$name" 2>/dev/null); then
      browser=$found
      break
    fi
  done
fi
mac_chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
if [ -z "$browser" ] && [ -f "$mac_chrome" ] && [ -x "$mac_chrome" ]; then
  browser=$mac_chrome
fi
if [ -z "$browser" ]; then
  case " $* " in
    *" open "*) echo "browse: no Chrome or Chromium found, so Playwright's own is used; \`sh scripts/browse.sh install-browser chromium\` downloads it once" >&2 ;;
  esac
fi

sandbox=true
if [ "$(id -u)" = 0 ]; then
  sandbox=false
fi

# node writes the JSON, so a path with a space or a quote in it stays one string.
mkdir -p "$WORK_DIR/.playwright"
node -e '
const [file, browser, sandbox, width, height, outputDir] = process.argv.slice(1);
const launchOptions = { chromiumSandbox: sandbox === "true" };
if (browser) launchOptions.executablePath = browser;
const viewport = { width: Number(width), height: Number(height) };
const config = { browser: { browserName: "chromium", launchOptions, contextOptions: { viewport } }, outputDir };
require("fs").writeFileSync(file, JSON.stringify(config, null, 2) + "\n");
' "$CONFIG" "$browser" "$sandbox" "$VIEWPORT_WIDTH" "$VIEWPORT_HEIGHT" "$WORK_DIR/out"

cd "$WORK_DIR"
exec "$CLI" "$@"
