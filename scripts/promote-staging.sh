#!/usr/bin/env bash
# Moves `staging`, every STAGING_EVERY_MINUTES (two hours), to a snapshot of main whose CI passed
# (#3275, the owners' call of 2026-11-05).
#
# Three branches, each a site's (docs/deploy-cloudflare.md, section 2):
#   main        every merge, through the merge queue and its five required checks. No site.
#   staging     this script's: fast-forwarded to main's newest commit whose push-to-main CI run
#               passed (with the merge-queue build of the same commit, every piece of ci.yml: the
#               push run runs what that build skipped, #3564). deploy-staging.yml deploys each push
#               of it to https://staging.jackioh.com.
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
# The freeze (the owners' call of 2026-11-10: production had waited four days on red super runs while
# staging kept taking main's newer commits, so each day's run found breakage the last fix had not
# seen). While the newest finished super run on staging failed, production waits on a fix, so staging
# holds still, on the schedule and on main's green CI runs alike, and the night bot fixes what it
# holds (super.yml's report hands it the issue titled GATE_TITLE). It moves again by itself only to
# test that fix: once the issue closes after the red run ended (the fix's pull request closes it on
# merge, or a person does), staging takes main's newest green commit pushed after the close, which
# carries the fix, without its two hours' wait, and the super run starts on it at once rather than
# the next day. A run that passes ends the freeze, and production promotes it; one still red opens a
# new issue, and staging holds again. While frozen:
#   - a super run on staging still running: staging holds, since that run is testing it;
#   - staging moved since the red run (by hand) and nothing is testing it: the super run starts on it;
#   - a red run with no issue (its report job never ran, as on 2026-11-09): this opens the issue and
#     hands it to the night bot as the report would, REPORT_GRACE_MINUTES after the run ended (the
#     issue search lags a new issue by a little).
# A manual run moves staging as ever, frozen or not.
#
# What runs it (EVENT is github.event_name):
#   schedule            every two hours: moves staging when it is due, and not frozen.
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
# The freeze (the header says why): the super run, the title of its issue for a red run on staging
# (super.yml's DAILY_TITLE), who fixes it (super.yml's BOT_LOGIN), and how long after a red run ends
# its issue may still be missing before this opens it.
: "${SUPER_WORKFLOW:=super.yml}"
: "${GATE_TITLE:=CI: daily super run failed}"
: "${BOT_LOGIN:=jgoetzmann-bot}"
: "${REPORT_GRACE_MINUTES:=15}"
export GATE_TITLE
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

# Seconds since the epoch of an ISO 8601 time.
epoch() { jq -rn --arg t "$1" '$t | sub("\\.[0-9]+"; "") | fromdateiso8601'; }

# Starts the super run on staging's commit. One that does not start is tried again by the next run
# of this, which finds staging moved since the red run with nothing testing it.
test_staging() {
  if gh workflow run "$SUPER_WORKFLOW" --ref "$STAGING_BRANCH"; then
    echo "Started the super run on $STAGING_BRANCH (\`${1:0:12}\`)."
    printf '### the super run starts on `%s` (`%s`)\n' "$STAGING_BRANCH" "${1:0:12}" >> "$SUMMARY"
  else
    echo "::warning::Could not start the super run on $STAGING_BRANCH; the next run of this tries again."
  fi
}

# Opens the issue a red run on staging should have had, handed to the night bot as super.yml's report
# hands it, when that report never ran.
report_red() {
  local url number
  url=$(gh issue create --title "$GATE_TITLE" --body "$(printf '%s\n' \
    "[The super run]($1) on \`$STAGING_BRANCH\` (\`${2:0:7}\`) failed, and no issue was opened for it: its \`report\` job did not run. The run's page names each failed job and links its log." \
    "" \
    "Night bot: this run is on \`$STAGING_BRANCH\`, so it is yours, and it comes first (\`priority:high\`): production waits on it, and \`$STAGING_BRANCH\` holds still until this closes (scripts/promote-staging.sh, the freeze). Root-cause every failed job from its log and fix them all in a pull request on \`main\` that closes this; then \`$STAGING_BRANCH\` takes main's first green commit after the close and the super run starts on it at once. A flake is not a root cause, so make a flaky test robust instead of re-running it, and never skip, disable or quarantine a test to get green. You cannot change \`.github/\`: make the part of the fix that lives elsewhere, and give a person the exact lines for the rest." \
    "" \
    "---" \
    "_Posted by \`scripts/promote-staging.sh\` in place of \`.github/workflows/super.yml\`'s \`report\`._")")
  number=${url##*/}
  echo "Opened #$number for the red run, which had no issue."
  gh issue edit "$number" --add-assignee "$BOT_LOGIN" --add-label "bot:build" \
    --add-label "priority:high" --add-label "difficulty:hard" \
    || echo "::warning::could not hand #$number to @$BOT_LOGIN"
}

# The freeze (the header says why). Sets `attempt` when staging should move to test a fix; exits when
# it holds still.
attempt=false cand=''
if [ "$EVENT" != workflow_dispatch ]; then
  last=$(gh run list --workflow "$SUPER_WORKFLOW" --branch "$STAGING_BRANCH" --limit 20 \
    --json status,conclusion,headSha,updatedAt,url --jq '
      ([.[] | select(.status != "completed")] | length) as $busy
      | ([.[] | select(.status == "completed" and .conclusion != "cancelled" and .conclusion != "skipped")][0] // {}) as $last
      | "\($busy) \($last.conclusion // "none") \($last.headSha // "-") \($last.updatedAt // "-") \($last.url // "-")"')
  read -r busy conclusion tested ended url <<<"$last"
  if [ "$conclusion" != success ] && [ "$conclusion" != none ]; then
    held="$STAGING_BRANCH holds still: the super run on it ended $conclusion on \`${tested:0:12}\` ($url), so production waits on a fix"
    if [ "$busy" != 0 ]; then
      echo "$held, and a super run on $STAGING_BRANCH is under way."
      exit 0
    fi
    if [ "$tested" != "$current" ]; then
      echo "$held. $STAGING_BRANCH has moved since, to \`${current:0:12}\`, and nothing is testing it."
      test_staging "$current"
      exit 0
    fi
    issue=$(gh issue list --state all --search "in:title \"$GATE_TITLE\" sort:created-desc" --limit 10 \
      --json number,title,state,closedAt --jq '
        map(select(.title == env.GATE_TITLE)) | sort_by(.number) | last // {}
        | "\(.number // "-") \(.state // "-") \(.closedAt // "-")"')
    read -r number state closed <<<"$issue"
    if [ "$state" = OPEN ]; then
      echo "$held: #$number. It moves once #$number closes."
      exit 0
    fi
    if [ "$closed" = - ] || [ "$(epoch "$closed")" -lt "$(epoch "$ended")" ]; then
      if [ $(((NOW - $(epoch "$ended")) / 60)) -lt "$REPORT_GRACE_MINUTES" ]; then
        echo "$held, and its issue is not open yet."
      else
        echo "$held, and no issue was opened for it."
        report_red "$url" "$tested"
      fi
      exit 0
    fi
    cand=$(green --created ">=$closed")
    if [ -z "$cand" ]; then
      echo "$held. #$number closed at $closed; it moves to main's first commit after that whose CI passes."
      exit 0
    fi
    attempt=true
    echo "#$number closed at $closed, after the red run: $STAGING_BRANCH moves now to test the fix."
  fi
fi

[ -n "$cand" ] || cand=$INPUT_SHA
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

# A fix under test goes at once (the freeze), as a manual run does.
if [ "$EVENT" != workflow_dispatch ] && [ "$attempt" = false ]; then
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
[ "$attempt" = false ] || test_staging "$cand"
{
  printf '### %s <- `%s`\n\n%s commit(s) of `%s` since `%s`:\n\n~~~\n' "$STAGING_BRANCH" "${cand:0:12}" "$count" "$MAIN_BRANCH" "${current:0:12}"
  git log --format='%h %s' "$current..$cand" | head -n 100
  echo '~~~'
} >> "$SUMMARY"
