#!/bin/sh
# Vercel's Ignored Build Step, run by vercel.json's `ignoreCommand`: exit 0 skips the build, exit 1
# builds. Vercel deploys on the LAST push, and only when the push can change what the site serves:
#
#   1. A commit message that says `[vercel]` builds, on any branch: the flag.
#   2. A branch that is not main (and not a production deploy) is skipped without the flag.
#   3. main builds unless every file changed since the last commit Vercel built sits on the skip list
#      below: paths the web bundle never reads (it imports packages/* and nothing else of the repo).
#
# Rule 3 diffs against VERCEL_GIT_PREVIOUS_SHA, the last commit Vercel built, and only when that
# commit is an ancestor of HEAD. A commit whose build was cancelled or failed (the daily deployment
# cap does that) is therefore never skipped past: the next push still diffs against what is really
# live. Anything that goes wrong, and any doubt (no previous sha, a shallow clone without it, an
# empty diff, a path off the list), builds. vercel.json only lets a clean exit 0 through.
#
# Adding a path to the list means checking that apps/web and packages/* (and the root files pnpm
# installs from) do not read it. apps/web/src/net/vercel-ignore.test.ts holds the list in place.

if printf '%s' "$VERCEL_GIT_COMMIT_MESSAGE" | grep -qiF '[vercel]'; then
  exit 1
fi

if [ "$VERCEL_ENV" != production ] && [ "$VERCEL_GIT_COMMIT_REF" != main ]; then
  exit 0
fi

prev=$VERCEL_GIT_PREVIOUS_SHA
[ -n "$prev" ] || exit 1
git merge-base --is-ancestor "$prev" HEAD 2>/dev/null || exit 1
files=$(git diff --name-only "$prev" HEAD 2>/dev/null) || exit 1
[ -n "$files" ] || exit 1

# The loop below splits `git diff`'s output on whitespace; no name may expand as a glob.
set -f
for file in $files; do
  case "$file" in
    bot/* | .harness/* | .github/* | docs/* | reviews/* | e2e/* | apps/server/* | scripts/*) ;;
    render.yaml) ;;
    CLAUDE.md | AGENTS.md | GEMINI.md | README.md | SPEC.md | BUILD.md | REVIEW.md) ;;
    ARCHITECTURE-CCG.md | JackiOh_Mechanics.md | JackiOh_Core_Cards.md | JackiOh_Classic_Cards.md | JackiOh_Tokens.md) ;;
    *) exit 1 ;;
  esac
done
exit 0
