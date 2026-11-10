#!/usr/bin/env bash
# Moves `staging`, every STAGING_EVERY_MINUTES (two hours), to a snapshot of main whose CI passed
# (#3275, the owners' call of 2026-11-05).
#
# Three branches, each a site's (docs/deploy-cloudflare.md, section 2):
#   main        every merge, through the merge queue and its five required checks. No site.
#   staging     this script's: fast-forwarded to main's newest commit whose push-to-main CI run
#               passed (ci.yml runs on a push to main every piece the merge queue's build of
#               the same commit did not pass already, so a green run means every piece passed
#               on that commit, there or in the queue). deploy-staging.yml
#               deploys each push of it to https://staging.jackioh.com.
#   production  promote-production.yml's: once a day, the commit of staging the daily super run
#               passed every job on (super.yml runs on staging, scripts/promote-production.sh).
# main moves many times an hour, so a gate that tests main chases a moving target; staging holds
# still between moves, which gives the daily super run one commit to pass and the night bot one
# commit to fix.
#
# The candidate is CAND: INPUT_SHA when someone asked for one (a manual run), else main's newest
# commit whose push-to-main CI run succeeded. It must be on main and have that green run. main is
# linear (every merge is a squash), so staging stays an ancestor of main and every commit it holds
# is a commit of main, which is what lets production's promotion and the issue name staging's
# commits as main's. Nothing moves staging backwards: a candidate staging already has is a no-op.
#
# The pace. A scheduled run fires every two hours, but GitHub fires this repository's schedules late,
# sometimes hours late (bot/README.md), so a green push-to-main CI run wakes this too, and then it
# moves staging only once the last push to staging is STAGING_EVERY_MINUTES old (read from the
# repository's activity, `repos/<repo>/activity`). A manual run never waits.
#
# It pushes CAND to staging with the checkout's credentials, which the workflow sets to
# BOT_GITHUB_TOKEN: GitHub refuses the Actions token a push that changes .github/workflows, and a push
# the Actions token makes starts no workflow, so deploy-staging.yml would never hear of it. The push
# is a fast-forward (the `staging` ruleset refuses any other, infra/github.tf). It writes
# `moved=true` and `sha=<CAND>` to GITHUB_OUTPUT when staging moved.
#
# apps/web/src/net/promote-staging.test.ts holds it, against a throwaway repository and a stand-in
# for `gh`.
#
# What runs it (EVENT is github.event_name):
#   schedule            every two hours: moves staging when it is due.
#   workflow_run        a green push-to-main CI run: the same, so a late schedule never holds it.
#   workflow_dispatch   by hand: INPUT_SHA, or main's newest green commit, at once.
# The backticks in the printf format below are Markdown for GitHub, not command substitutions.
# shellcheck disable=SC2016
set -Eeuo pipefail
trap 'echo "::error::scripts/promote-staging.sh line $LINENO: \`$BASH_COMMAND\` failed (exit $?)" >&2' ERR

: "${GITHUB_REPOSITORY:?}"
: "${EVENT:?}"
: "${INPUT_SHA:=}"
: "${MAIN_BRANCH:=main}"
: "${STAGING_BRANCH:=staging}"
# How long staging holds still between two moves, at least, unless someone asks.
: "${STAGING_EVERY_MINUTES:=120}"
# The clock, in seconds since the epoch.
: "${NOW:=$(date -u +%s)}"
REPO=$GITHUB_REPOSITORY
SUMMARY=${GITHUB_STEP_SUMMARY:-/dev/null}

fail() { echo "::error::$*" >&2; exit 1; }

case $EVENT in
  schedule | workflow_run | workflow_dispatch) ;;
  *) fail "promote-staging.sh does not handle the $EVENT event." ;;
esac

git fetch -q origin "$MAIN_BRANCH" "$STAGING_BRANCH" 2>/dev/null \
  || git fetch -q origin "$MAIN_BRANCH"
git rev-parse -q --verify "origin/$STAGING_BRANCH" >/dev/null \
  || fail "The $STAGING_BRANCH branch does not exist. Create it at a commit of $MAIN_BRANCH whose CI passed (git push origin <sha>:refs/heads/$STAGING_BRANCH); docs/deploy-cloudflare.md, section 2."
current=$(git rev-parse "origin/$STAGING_BRANCH")

# A commit of main with a green push-to-main CI run: "" for none.
green() {
  gh run list --workflow ci.yml --branch "$MAIN_BRANCH" --event push --status success --limit 1 "$@" \
    --json headSha --jq '.[0].headSha // ""'
}

cand=$INPUT_SHA
[ -n "$cand" ] || cand=$(green)
[ -n "$cand" ] || fail "No push to $MAIN_BRANCH has a passing CI run, so there is nothing to move $STAGING_BRANCH to."
cand=$(git rev-parse -q --verify "$cand^{commit}") || fail "\`$INPUT_SHA\` is not a commit."
git merge-base --is-ancestor "$cand" "origin/$MAIN_BRANCH" || fail "\`${cand:0:12}\` is not on $MAIN_BRANCH."
[ -n "$(green --commit "$cand")" ] || fail "\`${cand:0:12}\` has no passing push-to-$MAIN_BRANCH CI run."
# An older commit (a CI run passing again, by hand, on an old commit makes it the newest green run)
# is a no-op, never a step back.
if git merge-base --is-ancestor "$cand" "$current"; then
  echo "$STAGING_BRANCH (\`${current:0:12}\`) already has ${cand:0:12}; nothing to move."
  exit 0
fi
git merge-base --is-ancestor "$current" "$cand" \
  || fail "$STAGING_BRANCH (\`${current:0:12}\`) is not an ancestor of \`${cand:0:12}\`: it holds something $MAIN_BRANCH does not, or $MAIN_BRANCH was rewritten. Not moving it; compare them with git log --oneline --graph origin/$MAIN_BRANCH origin/$STAGING_BRANCH."

if [ "$EVENT" != workflow_dispatch ]; then
  pushed=$(gh api "repos/$REPO/activity?ref=refs/heads/$STAGING_BRANCH&per_page=1" --jq '.[0].timestamp // ""' 2>/dev/null || true)
  if [ -n "$pushed" ] && at=$(jq -rn --arg t "$pushed" '$t | sub("\\.[0-9]+"; "") | fromdateiso8601' 2>/dev/null); then
    age=$(((NOW - at) / 60))
    if [ "$age" -lt "$STAGING_EVERY_MINUTES" ]; then
      echo "$STAGING_BRANCH moved $age minute(s) ago; it moves again once it is $STAGING_EVERY_MINUTES old (to ${cand:0:12} or newer)."
      exit 0
    fi
  else
    echo "::notice::When $STAGING_BRANCH was last pushed could not be read; moving it now."
  fi
fi

count=$(git rev-list --count "$current..$cand")
err=$(mktemp)
if ! git push origin "$cand:refs/heads/$STAGING_BRANCH" 2>"$err"; then
  fail "Could not push ${cand:0:12} to $STAGING_BRANCH: $(tr '\n' ' ' <"$err" | cut -c1-600). Moving it past a change to .github/workflows needs a token with the workflow scope: the BOT_GITHUB_TOKEN secret (bot/README.md, 'Setting it up'), which promote-staging.yml checks out with."
fi
rm -f "$err"
{
  echo "moved=true"
  echo "sha=$cand"
} >> "${GITHUB_OUTPUT:-/dev/null}"
echo "Moved $STAGING_BRANCH from ${current:0:12} to ${cand:0:12} ($count commit(s)); deploy-staging.yml deploys it."
{
  printf '### %s <- `%s`\n\n%s commit(s) of `%s` since `%s`:\n\n~~~\n' "$STAGING_BRANCH" "${cand:0:12}" "$count" "$MAIN_BRANCH" "${current:0:12}"
  git log --format='%h %s' "$current..$cand" | head -n 100
  echo '~~~'
} >> "$SUMMARY"
