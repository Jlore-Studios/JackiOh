#!/bin/sh
# `sh scripts/ci-scope.sh <base> <head>`: prints `full=true` or `full=false`, the line ci.yml's
# `changes` job appends to $GITHUB_OUTPUT.
#
# `full=false` means every file the pull request changes sits on the SKIP list below, which holds
# only paths that no CI job other than `bot selftest` ever reads (that check has its own workflow and
# always runs): the night bot's code and its switches, the workflows that run the bot or watch the
# repo, and the agent instruction files (CLAUDE.md, AGENTS.md, GEMINI.md: code cites them in comments
# and nothing opens them). Nothing a test, a build or a type check reads is on the list. The other
# docs are not: tests read SPEC.md, BUILD.md and docs/ (rulings.test.ts, spec-rows.test.ts,
# rules.test.ts, ...).
#
# Anything else, and any doubt, is `full=true`: an empty diff, an unreadable one, a missing argument.
# ci.yml never calls this for a push to main, which always runs everything.
#
# Adding a path to the list means checking, first, that nothing in packages/, apps/, e2e/ or the
# workflow's other jobs reads it. apps/web/src/net/ci-scope.test.ts holds the list in place.

base=$1
head=$2

# The loop below splits `git diff`'s output on whitespace; no name may expand as a glob.
set -f

if [ -z "$base" ] || [ -z "$head" ]; then
  echo "full=true"
  exit 0
fi

files=$(git diff --name-only "$base...$head" 2>/dev/null) || files=
if [ -z "$files" ]; then
  echo "ci-scope: no readable diff for $base...$head, running everything" >&2
  echo "full=true"
  exit 0
fi

full=false
for file in $files; do
  case "$file" in
    bot/* | .harness/* | .squishy/*) ;;
    CLAUDE.md | AGENTS.md | GEMINI.md) ;;
    .github/workflows/bot-commands.yml | .github/workflows/bot-night.yml | .github/workflows/bot-selftest.yml) ;;
    .github/workflows/squishy-run.yml | .github/workflows/squishy-commands.yml) ;;
    .github/workflows/triage.yml | .github/workflows/deploy-watch.yml | .github/workflows/ci-duration.yml) ;;
    *)
      echo "ci-scope: $file is not on the skip list, running everything" >&2
      full=true
      break
      ;;
  esac
done

echo "full=$full"
