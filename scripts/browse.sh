#!/bin/sh
# `sh scripts/browse.sh <command> [args]`: a headless browser an agent drives one shell command at a
# time to look at the web client, for UI work only (.claude/skills/ui-walkthrough/SKILL.md, #811).
# Every argument goes to Microsoft's Playwright CLI (`@playwright/cli`): `open <url>`, `goto`,
# `click`, `eval`, `resize`, `screenshot`, `close`, … (`sh scripts/browse.sh --help` lists them). The
# browser stays open between commands, until `close`.
#
# It costs nothing to anyone who never runs it, which is the point:
#   - the CLI is installed on first use into .cache/playwright-cli/ (gitignored, as build-wasm.sh
#     keeps wasm-bindgen in .cache/bin/) with npm, outside the pnpm workspace, so no package.json
#     or lockfile names it and no `pnpm install` (CI's, the night bot's, yours) fetches it. The
#     install goes into a folder of its own and is published as .cache/playwright-cli/<version>, a
#     symlink made only once that install is complete, so a run killed half-way or two first runs
#     at once never leave a broken install that a later run trusts. A run that finds the link
#     installs nothing. The CLI's daily check for a newer version is off (NO_UPDATE_NOTIFIER): its
#     notice tells you to add the CLI to package.json, the very thing this script exists not to do;
#   - no browser is downloaded unless asked for (`sh scripts/browse.sh install-browser chromium`).
#     `open` picks one, in order: JACKIOH_BROWSER (a path to a Chrome or Chromium executable); the
#     Chromium a Claude Code cloud container ships at $PLAYWRIGHT_BROWSERS_PATH/chromium; a Chrome
#     or Chromium on PATH, or macOS's Google Chrome; else Playwright's own, which install-browser
#     downloads once.
#
# The CLI runs from .cache/browse/ (gitignored), where `open` writes the config it reads
# (.playwright/cli.config.json: the browser, a VIEWPORT_WIDTH x VIEWPORT_HEIGHT page, and Chromium's
# sandbox off when running as root, as a cloud container does, since Chromium refuses root with it
# on; otherwise the CLI's own choice). So a relative path, `screenshot --filename=board.png` among
# them, means .cache/browse/board.png, and the CLI's own snapshots and console logs land in
# .cache/browse/out/; an absolute path, or one climbing out with `..`, goes where it says.

set -eu

PLAYWRIGHT_CLI_VERSION="0.1.22"
# The page size a browser opens at: BUILD's desktop viewport. `resize <w> <h>` changes it after.
VIEWPORT_WIDTH=1280
VIEWPORT_HEIGHT=720

ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
INSTALL_ROOT="$ROOT/.cache/playwright-cli"
INSTALL="$INSTALL_ROOT/$PLAYWRIGHT_CLI_VERSION"
CLI="$INSTALL/node_modules/.bin/playwright-cli"
WORK_DIR="$ROOT/.cache/browse"
CONFIG="$WORK_DIR/.playwright/cli.config.json"

export NO_UPDATE_NOTIFIER=1

for tool in node npm; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "browse: $tool is not on PATH. Install Node (.nvmrc pins the version)." >&2
    exit 1
  fi
done

# The CLI, pinned. Its own dependencies (playwright, playwright-core) are exact versions, so the
# pin fixes the whole install.
if [ ! -x "$CLI" ]; then
  echo "browse: installing @playwright/cli@$PLAYWRIGHT_CLI_VERSION into .cache/playwright-cli/ (once)" >&2
  mkdir -p "$INSTALL_ROOT"
  # A link whose folder was deleted by hand points nowhere; a link that points somewhere is another
  # run's finished install and stays.
  if [ -L "$INSTALL" ] && [ ! -e "$INSTALL" ]; then
    rm -f "$INSTALL"
  fi
  staging="$INSTALL.$$"
  rm -rf "$staging"
  # An interrupted install removes its folder and stops; without the exit, sh would carry on and
  # link the folder it just removed.
  trap 'rm -rf "$staging"' EXIT
  trap 'exit 130' INT TERM
  mkdir "$staging"
  PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm install --prefix "$staging" --no-save --no-audit --no-fund \
    --loglevel=error "@playwright/cli@$PLAYWRIGHT_CLI_VERSION" >&2
  # `ln -sn` fails when the link exists: another run finished first, and its install is used. (Without
  # -n, ln would follow that link and make this one inside the other run's folder.)
  if ! ln -sn "${staging##*/}" "$INSTALL" 2>/dev/null; then
    rm -rf "$staging"
  fi
  trap - EXIT INT TERM
fi

# The command: the first argument that is not an option (`-s=<session>`, `--help`, …).
command=
for arg in "$@"; do
  case $arg in
    -*) ;;
    *)
      command=$arg
      break
      ;;
  esac
done

# `open` starts the browser, and only it reads the config, so only it writes one.
if [ "$command" = open ]; then
  browser=${JACKIOH_BROWSER:-}
  if [ -n "$browser" ]; then
    # The CLI runs from .cache/browse/, so a relative path is made absolute here.
    case $browser in
      /*) ;;
      *) browser="$PWD/$browser" ;;
    esac
    if [ ! -f "$browser" ] || [ ! -x "$browser" ]; then
      echo "browse: JACKIOH_BROWSER is \"$browser\", which is not an executable" >&2
      exit 1
    fi
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
    echo "browse: no Chrome or Chromium found, so Playwright's own is used; \`sh scripts/browse.sh install-browser chromium\` downloads it once" >&2
  fi

  root=false
  if [ "$(id -u)" = 0 ]; then
    root=true
  fi

  # node writes the JSON, so a path with a space or a quote in it stays one string, and renames it
  # into place, so a session opening at the same moment never reads half a file.
  mkdir -p "$WORK_DIR/.playwright"
  node -e '
const [file, browser, root, width, height, outputDir] = process.argv.slice(1);
const fs = require("fs");
const launchOptions = {};
if (browser) launchOptions.executablePath = browser;
if (root === "true") launchOptions.chromiumSandbox = false;
const viewport = { width: Number(width), height: Number(height) };
const config = { browser: { browserName: "chromium", launchOptions, contextOptions: { viewport } }, outputDir };
fs.writeFileSync(`${file}.${process.pid}`, JSON.stringify(config, null, 2) + "\n");
fs.renameSync(`${file}.${process.pid}`, file);
' "$CONFIG" "$browser" "$root" "$VIEWPORT_WIDTH" "$VIEWPORT_HEIGHT" "$WORK_DIR/out"
fi

mkdir -p "$WORK_DIR"
cd "$WORK_DIR"
exec "$CLI" "$@"
