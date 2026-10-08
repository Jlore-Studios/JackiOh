#!/bin/sh
# `sh scripts/ci-scope.sh <base> <head>`: prints two lines, `full=true|false` and `db=true|false`,
# which ci.yml's `changes` job appends to $GITHUB_OUTPUT.
#
# `full=false` means every file the pull request changes sits on the SKIP list below, which holds
# only paths that no CI job other than `bot selftest` ever reads (that check has its own workflow and
# always runs): the night bot's code and its switches, the workflows that run the bot or watch the
# repo (the pull request title check among them), the agent instruction files (CLAUDE.md, AGENTS.md,
# GEMINI.md, CONTRIBUTING.md: code cites them in comments and nothing opens them), the repository's
# meta that only GitHub reads (the issue forms, the pull request template, CODEOWNERS), and the
# training lanes' prompts and loop (training/*.md, training/loop.sh: Devin and the training box read
# them, no job does). Nothing a test, a build or a check reads is on the list. The other docs are
# not: `spec check` reads spec/, the radiant-standard test reads docs/radiant-audit.md, web tests
# read docs/ and SPEC.md, and the `training gate` job reads training/history/.
#
# `db=true` means the pull request also touches what the `db` job tests against a real Postgres
# (docs/v0.3.0/README.md §7): the migrations, the store, the Dockerfile and render.yaml, plus that
# job's own tests and the lockfile and manifest that pick sqlx. Everything else leaves the job to the
# daily super run, and its skip passes the check.
#
# Anything else, and any doubt, runs everything (`full=true`, `db=true`): an empty diff, an
# unreadable one, a missing argument. ci.yml never calls this for a push or a dispatch, which always
# run everything.
#
# Adding a path to the skip list means checking, first, that nothing in crates/, apps/, e2e/, spec/
# or the workflow's other jobs reads it. apps/web/src/net/ci-scope.test.ts holds both lists in place.

base=$1
head=$2

# The loop below splits `git diff`'s output on whitespace; no name may expand as a glob.
set -f

if [ -z "$base" ] || [ -z "$head" ]; then
  echo "full=true"
  echo "db=true"
  exit 0
fi

# --no-renames: a move lists both paths, so a file moved onto the skip list still counts where it was.
files=$(git diff --no-renames --name-only "$base...$head" 2>/dev/null) || files=
if [ -z "$files" ]; then
  echo "ci-scope: no readable diff for $base...$head, running everything" >&2
  echo "full=true"
  echo "db=true"
  exit 0
fi

full=false
for file in $files; do
  case "$file" in
    bot/* | .harness/* | .squishy/*) ;;
    CLAUDE.md | AGENTS.md | GEMINI.md | CONTRIBUTING.md) ;;
    .github/ISSUE_TEMPLATE/* | .github/pull_request_template.md | .github/CODEOWNERS) ;;
    training/*.md | training/loop.sh) ;;
    .github/workflows/bot-commands.yml | .github/workflows/bot-night.yml | .github/workflows/bot-selftest.yml) ;;
    .github/workflows/squishy-run.yml | .github/workflows/squishy-commands.yml) ;;
    .github/workflows/triage.yml | .github/workflows/deploy-watch.yml | .github/workflows/ci-duration.yml) ;;
    .github/workflows/pr-title.yml) ;;
    *)
      echo "ci-scope: $file is not on the skip list, running everything" >&2
      full=true
      break
      ;;
  esac
done

db=false
if [ "$full" = true ]; then
  for file in $files; do
    case "$file" in
      crates/server/migrations/* | crates/server/src/db/* | crates/server/Dockerfile | render.yaml | \
        crates/server/tests/sql/* | crates/server/tests/db/* | crates/server/tests/deploy/* | crates/server/tests/store/* | \
        crates/server/Cargo.toml | Cargo.lock)
        echo "ci-scope: $file is read by the db job" >&2
        db=true
        break
        ;;
    esac
  done
fi

echo "full=$full"
echo "db=$db"
