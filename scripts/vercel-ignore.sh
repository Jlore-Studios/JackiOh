#!/bin/sh
# Vercel's Ignored Build Step, run by vercel.json's `ignoreCommand`: exit 0 skips the build, exit 1
# builds. Vercel deploys on the LAST push, and only when the push can change what the site serves:
#
#   1. A commit message that says `[vercel]` builds, on any branch: the flag.
#   2. A branch that is not main (and not a production deploy) is skipped without the flag.
#   3. main builds unless every file changed since the last commit Vercel built sits on the skip list
#      below: paths the web bundle never reads. The bundle is apps/web plus the WASM module that
#      scripts/build-wasm.sh compiles from crates/{engine,cards,ai,wasm} (with the workspace's
#      Cargo.toml, Cargo.lock, every member's manifest, rust-toolchain.toml and .cargo/), and the
#      card data it imports from crates/cards. The server and the CLI (crates/server, crates/tools)
#      are never compiled into it, only their manifests are read (cargo loads the whole workspace);
#      integration tests (a crate's tests/), web test files (*.test.ts, *.test.tsx, apps/web/src/test/),
#      tooling and READMEs are never imported, so a commit that touches only those leaves the site
#      byte for byte what it was and does not deploy.
#
# Rule 3 diffs against VERCEL_GIT_PREVIOUS_SHA, the last commit Vercel built, and only when that
# commit is an ancestor of HEAD. A commit whose build was cancelled or failed (the daily deployment
# cap does that) is therefore never skipped past: the next push still diffs against what is really
# live. Anything that goes wrong, and any doubt (no previous sha, a shallow clone without it, an
# empty diff, a path off the list), builds. vercel.json only lets a clean exit 0 through.
#
# Adding a path to the list means checking that apps/web, the four crates the WASM module is built
# from (and the root files pnpm and cargo read) do not read it. A name alone is not proof:
# crates/cards/src/scripts/ holds the card scripts, which ARE compiled in, and scripts/build-wasm.sh
# builds the module and scripts/vercel-install.sh installs its toolchain, so both are carved out of
# scripts/ above the line that skips the rest.
# apps/web/src/net/vercel-ignore.test.ts holds the list in place.

if printf '%s' "$VERCEL_GIT_COMMIT_MESSAGE" | grep -qiF '[vercel]'; then
  exit 1
fi

if [ "$VERCEL_ENV" != production ] && [ "$VERCEL_GIT_COMMIT_REF" != main ]; then
  exit 0
fi

prev=$VERCEL_GIT_PREVIOUS_SHA
[ -n "$prev" ] || exit 1
git merge-base --is-ancestor "$prev" HEAD 2>/dev/null || exit 1
# --no-renames: a move lists both paths, so a file moved onto the skip list still counts where it was.
files=$(git diff --no-renames --name-only "$prev" HEAD 2>/dev/null) || exit 1
[ -n "$files" ] || exit 1

# The loop below splits `git diff`'s output on whitespace; no name may expand as a glob. The first
# case that matches wins, so each file the build reads inside a skipped directory comes first.
set -f
for file in $files; do
  case "$file" in
    scripts/build-wasm.sh | scripts/vercel-install.sh | crates/server/Cargo.toml | crates/tools/Cargo.toml) exit 1 ;;
    bot/* | .harness/* | .squishy/* | .github/* | docs/* | reviews/* | e2e/* | spec/* | training/* | scripts/*) ;;
    crates/server/* | crates/tools/* | render.yaml) ;;
    crates/engine/tests/* | crates/cards/tests/* | crates/ai/tests/* | crates/wasm/tests/*) ;;
    apps/web/*.test.ts | apps/web/*.test.tsx | apps/web/src/test/* | apps/web/scripts/* | apps/web/README.md) ;;
    CLAUDE.md | AGENTS.md | GEMINI.md | README.md | SPEC.md | BUILD.md | REVIEW.md) ;;
    ARCHITECTURE-CCG.md | JackiOh_Mechanics.md | JackiOh_Core_Cards.md | JackiOh_Classic_Cards.md | JackiOh_Tokens.md) ;;
    *) exit 1 ;;
  esac
done
exit 0
