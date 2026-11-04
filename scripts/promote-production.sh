#!/usr/bin/env bash
# Moves production, once a day by itself and whenever someone asks, never past a red super run.
#
# The daily promotion. Every day super.yml runs on main (its `schedule`). When a run on main passes
# every job, the commit it tested goes to production: this script opens a pull request of it into
# production and merges it at once, since the run that gates production already passed on exactly
# that commit, and the workflow's `deploy` job then puts it on jackioh.com, the site and its game
# server together (wrangler.jsonc). Each such run is acted on once (the issue's `daily`). A red run
# on main is the night bot's to fix (super.yml's `report`, `priority:high`), and production waits
# for the next run that passes; a promotion or a deploy that fails is Squishy's (promote-production.yml's
# `report`). The owners asked for it on 2026-11-03, after a day of production by hand only (#2750).
#
# By hand, sooner. A person comments `/promote` on the one issue labeled RELEASE_LABEL (or runs
# promote-production.yml from the Actions tab); this opens a pull request of a commit of main into
# production, super.yml runs on that pull request, and it merges once every one of its jobs passed.
# docs/deploy-cloudflare.md is the plan and the runbook, and apps/web/src/net/promote-production.test.ts
# holds this in place, against a throwaway repository and a stand-in for `gh`.
#
# Staging is not this script's: deploy-staging.yml deploys every commit of main whose push-to-main CI
# passed, by itself, and nothing here holds or hurries it.
#
# The production issue. One issue labeled RELEASE_LABEL is open at a time. Its title says where
# production stands (everything live, commits waiting, or a candidate under test or red); its body
# lists what main has that is not live yet (the queue) and the commands. Its first line is a hidden
# state line: the last comment read (`last`), the last red super run it reported (`reported`) and
# the last passing super run on main it promoted from (`daily`). Once it is ROTATE_DAYS old, a fresh
# issue takes its place with the same state line, and the old one is retitled with the days it
# covered and closed with a link to the new one, so no thread gathers every comment and retitle
# (#1543 had 43 comments in two weeks). A command someone still leaves on a closed one is carried out
# all the same (COMMENT_ISSUE) and answered on the open one.
# Anyone with write access comments on it:
#   /promote [sha]         opens the promotion pull request: main's newest commit whose push-to-main
#                          CI passed, or that commit of main. `/fast-forward` says the same.
#   /skip-super <reason>   the emergency way: merges the open promotion pull request, or main's newest
#                          green commit, at once, without the super run. The reason is required; the
#                          issue and the pull request record who, why and which commit.
#   /cancel                closes the open promotion pull request; nothing merges.
# `/hold`, `/resume` and `/delay`, from the countdown before 2026-11-02, are answered that there is
# nothing to hold: production follows the super run. Each command gets a +1 reaction, or a confused
# one when it was refused, and a reply saying what came of it.
#
# The promotion pull request. The candidate is pushed as promote/<date>-<sha> and a pull request into
# production opened. super.yml runs on every pull request into production; its `report` job hands a
# red run to the night bot as `priority:high` and `difficulty:hard`. A newer `/promote` closes the open
# one as superseded. Every run of this script then reconciles: when the open pull request's newest
# super run on its head commit passed, it merges it with a merge commit; when that run failed, it says
# so on the issue, once a run attempt. The pull request stays open after a failure, so re-running the
# run's failed jobs merges it once they pass, and the next daily promotion of a newer commit
# supersedes it. The daily promotion's own pull request is opened with GH_TOKEN, which starts no
# workflow, so super does not run on it a second time. Because production only ever receives merges of main
# commits, its files always equal the candidate's; a production that carries anything else (someone
# committed to it) stops the run before anything is opened, and so does a merge that leaves production
# different from the candidate. Nothing is ever force-pushed. If production does not exist, only
# `/skip-super` makes it: the candidate is pushed as production, with no pull request.
#
# What runs it (EVENT is github.event_name):
#   issue_comment       a command on the production issue.
#   workflow_run        a passing super run on main, which is the daily promotion; a finished super run
#                       on a promotion pull request, which is when one merges; or a green CI run on
#                       main, which brings the queue up to date. Every run of any kind does the daily
#                       promotion and reconciles, so a wake-up that GitHub's concurrency group dropped
#                       (it keeps one waiting run, not all) is made up by the next.
#   workflow_dispatch   by hand: `/promote` for INPUT_SHA, or the newest green commit; with
#                       INPUT_SKIP_REASON, `/skip-super` for it, recorded as ACTOR's.
#
# When production moves it writes `promoted=true` to GITHUB_OUTPUT, which is what starts the
# workflow's `deploy` job (GitHub Actions builds production's bundle and deploys it and the server's
# image with wrangler; Cloudflare builds nothing).
#
# It talks to GitHub through `gh` and to git through the checkout's own credentials: the workflow
# checks out main with its whole history. Two tokens. What moves production (the push of the promote
# branch, its pull request, closing it, the merge, the branch's deletion) goes out as PROMOTE_TOKEN,
# and the checkout pushes with the same one: whenever main changed a file under .github/workflows
# since production, GitHub refuses that ref update from the Actions token ("refusing to allow a GitHub
# App to create or update workflow ... without `workflows` permission"), and no `permissions:` entry
# can grant it. The workflow passes BOT_GITHUB_TOKEN, which has the `workflow` scope (bot/README.md).
# It is also why super.yml runs on the pull request at all: a pull request the Actions token opens
# starts no workflow. Everything else (the issue, its comments, reactions and the label, and reading
# pull requests and runs) stays on GH_TOKEN, the Actions token, whose writes start no workflow, so the
# night bot never sees the issue; so does the daily promotion's pull request, which super already
# passed on, so that it starts no second super run. The bot's pull request starts no loop: CI and super run on it as on
# any pull request into production, and neither starts this script except through workflow_run, which
# only reconciles; the push of a promote branch or of production starts nothing (every workflow's
# `push` names branches); and the bot's harness ignores its own events.

# The backticks in the printf formats below are Markdown for GitHub, not command substitutions.
# shellcheck disable=SC2016
set -Eeuo pipefail
# A command that fails on its own (not through `fail`) still leaves an annotation saying which.
trap 'echo "::error::scripts/promote-production.sh line $LINENO: \`$BASH_COMMAND\` failed (exit $?)" >&2' ERR

: "${GITHUB_REPOSITORY:?}"
: "${EVENT:?}"
: "${INPUT_SHA:=}"
: "${INPUT_SKIP_REASON:=}"
: "${RELEASE_LABEL:=production merge}"   # keep equal to the label named in promote-production.yml's `if`
# Who the production issue and the promotion pull requests are assigned to, and the label that keeps
# the night bot off them (docs/issues-and-patches.md: people's work is `human`, for both of them).
: "${ASSIGNEES:=jgoetzmann,MaxGoetzmann}"
: "${HUMAN_LABEL:=human}"
# The workflow that tests a promotion pull request, and the title its `report` job gives the issue a
# red run on one opens for the night bot (super.yml's CANDIDATE_TITLE).
: "${SUPER_WORKFLOW:=super.yml}"
: "${SUPER_TITLE:=CI: super run failed on a production candidate}"
# The branch whose passing super runs production follows (the daily promotion).
: "${MAIN_BRANCH:=main}"
: "${LISTED_COMMITS:=100}"
# How many days one production issue lasts before a fresh one takes over (the header says why).
: "${ROTATE_DAYS:=7}"
# The clock, in seconds since the epoch: when the open issue is old enough to hand over.
: "${NOW:=$(date -u +%s)}"
# The issue the comment that woke this run is on (the workflow's github.event.issue.number): a
# production issue that has since closed when it is not ISSUE.
: "${COMMENT_ISSUE:=}"
: "${MERGE_ATTEMPTS:=4}"
: "${MERGE_RETRY_SECONDS:=5}"
: "${PROMOTE_TOKEN:=${GH_TOKEN:-}}"
# The patch history whose newest entry names the catalog version a commit ships (crates/cards/build.rs
# compiles the same entry into the server and the bundle).
: "${PATCHES_JSON:=crates/cards/patches/patches.json}"
REPO=$GITHUB_REPOSITORY
SUMMARY=${GITHUB_STEP_SUMMARY:-/dev/null}

fail() { echo "::error::$*" >&2; exit 1; }
# Runs a git or gh command; when it fails, the error names it and quotes what it printed. Its output
# goes to stdout as usual. (Logs need a download; an annotation shows on the run page and in the API.)
try() {
  local err rc
  err=$(mktemp)
  if "$@" 2>"$err"; then rm -f "$err"; return 0; else rc=$?; fi
  local what="$1 $2"
  [ "$1" != promoter ] || what="gh $2"
  fail "\`$what\` failed (exit $rc): $(tr '\n' ' ' <"$err" | cut -c1-600)"
}
run_link() { [ -n "${GITHUB_RUN_ID:-}" ] && echo "([run](${GITHUB_SERVER_URL:-https://github.com}/$REPO/actions/runs/$GITHUB_RUN_ID))" || true; }

# --- git -------------------------------------------------------------------------------------

# The catalog version commit $1 ships, for the pull request and the issue: its PATCHES_JSON's newest
# entry, or nothing when it has none.
catalog() {
  git show "$1:$PATCHES_JSON" 2>/dev/null | jq -r '.[-1].version // empty' 2>/dev/null || true
}
has_production() { git rev-parse --verify -q origin/production >/dev/null; }
# What moves production goes out as PROMOTE_TOKEN (the header says why); the git push uses the
# checkout's credentials, which the workflow sets to the same token.
promoter() { GH_TOKEN=$PROMOTE_TOKEN gh "$@"; }
push_ref() {
  local err
  err=$(mktemp)
  if git push origin "$1" 2>"$err"; then rm -f "$err"; return 0; fi
  fail "Could not push $1: $(tr '\n' ' ' <"$err" | cut -c1-600). When main has changed .github/workflows since production, only a token with the workflow scope may move it: the BOT_GITHUB_TOKEN secret (bot/README.md, 'Setting it up'), which promote-production.yml checks out with."
}
# production moved: the workflow's `deploy` job builds it and deploys it only then.
moved() { echo "promoted=true" >> "${GITHUB_OUTPUT:-/dev/null}"; }
# Stops when production carries anything main does not (someone committed to it), before anything
# is opened or merged. $1 is the candidate.
same_history() {
  local current base
  current=$(git rev-parse origin/production)
  base=$(git merge-base "$current" "$1") || fail "production and $1 share no history."
  git diff --quiet "$base" "$current" || fail "production has diverged: it carries changes that are not on main (git diff ${base:0:12} ${current:0:12}). Not merging; see docs/deploy-cloudflare.md, 'production has diverged'."
}

newest_green() {
  gh run list --workflow ci.yml --branch main --event push --status success --limit 1 \
    --json headSha --jq '.[0].headSha // ""'
}
# CAND: the full sha of a commit of main whose push-to-main CI run passed, $1 or the newest. When it
# is refused, returns 1 with WHY_NOT saying why.
vet() {
  local c=$1
  CAND='' WHY_NOT=''
  [ -n "$c" ] || c=$(newest_green)
  if [ -z "$c" ]; then WHY_NOT="No push to main has a passing CI run, so nothing can be promoted."; return 1; fi
  if ! c=$(git rev-parse --verify -q "$c^{commit}"); then WHY_NOT="\`$1\` is not a commit."; return 1; fi
  if ! git merge-base --is-ancestor "$c" origin/main; then WHY_NOT="\`${c:0:12}\` is not on main."; return 1; fi
  if [ -z "$(gh run list --workflow ci.yml --branch main --event push --status success \
    --commit "$c" --limit 1 --json headSha --jq '.[].headSha')" ]; then
    WHY_NOT="\`${c:0:12}\` has no passing push-to-main CI run."
    return 1
  fi
  CAND=$c
}

# Only when production is moved with the Actions token (no BOT_GITHUB_TOKEN secret): GitHub refuses
# that token a push whose .github/workflows differ from the default branch's, which is the case while
# main's newest commit changes a workflow and its CI is still running, so the newest green commit is
# older. The push of promote/<sha> was refused on 2026-10-04 for exactly that. True (and
# RESULT=waiting) when commit $1 is such a commit. The bot's token has the workflow scope and never
# waits.
must_wait() {
  [ "$PROMOTE_TOKEN" = "${GH_TOKEN:-}" ] || return 1
  git diff --quiet "$1" origin/main -- .github/workflows && return 1
  RESULT=waiting
  echo "::notice::Not promoting ${1:0:12} yet: main has changed .github/workflows since it (newest $(git rev-parse --short origin/main)), and GitHub refuses this token a push of workflow files that differ from main's."
}

# --- the promotion pull request ----------------------------------------------------------------

# The open promotion pull request into production, as "number branch head-sha", or nothing.
pending() {
  try gh pr list --base production --state open --limit 100 --json number,headRefName,headRefOid \
    --jq '[.[] | select(.headRefName | startswith("promote/"))] | sort_by(.number) | last // empty | "\(.number) \(.headRefName) \(.headRefOid)"'
}

# Opens the promotion pull request for commit CAND of main. Sets RESULT to opened, testing (one is
# open for it already), uptodate, waiting or noproduction, and PR_NUMBER, BRANCH, COUNT and CATALOG.
# With TESTED set (the link of a super run on main that passed on CAND), it is the daily promotion's:
# opened with GH_TOKEN, whose pull request starts no workflow, since super already passed on CAND.
open_candidate() {
  local cand=$CAND current p num head sha body url opener=promoter
  PR_NUMBER='' BRANCH='' COUNT=0 CATALOG=$(catalog "$cand")
  if ! has_production; then RESULT=noproduction; return 0; fi
  current=$(git rev-parse origin/production)
  if git merge-base --is-ancestor "$cand" "$current"; then
    RESULT=uptodate
    echo "production already has ${cand:0:12}; nothing to merge."
    return 0
  fi
  same_history "$cand"
  must_wait "$cand" && return 0

  p=$(pending)
  if [ -n "$p" ]; then
    read -r num head sha <<<"$p"
    if [ "$sha" = "$cand" ]; then
      RESULT=testing PR_NUMBER=$num BRANCH=$head
      echo "#$num already promotes ${cand:0:12}."
      return 0
    fi
    try promoter pr close "$num" --comment "Superseded by the promotion of \`${cand:0:12}\`, a newer commit of main."
    promoter api -X DELETE "repos/$REPO/git/refs/heads/$head" --silent || true
    echo "Closed #$num, superseded."
  fi

  COUNT=$(git rev-list --count "$current..$cand")
  BRANCH="promote/$(date -u +%Y%m%d)-${cand:0:7}"
  body=$(mktemp)
  {
    echo "Promotes \`main\` up to \`${cand:0:12}\` into \`production\`, which is what players are served: the site and its server, together."
    echo
    if [ -n "${TESTED:-}" ]; then
      echo "The daily promotion: the super run on \`main\` passed every job on this commit ($TESTED), so this merges at once, with no second super run, and \`promote-production.yml\` then deploys it."
    else
      echo "The daily super run (\`super.yml\`) tests this pull request. It merges once every job passes, and \`promote-production.yml\` then deploys it. A red run hands its fix to the night bot (\`priority:high\`); this stays open, and re-running the failed jobs merges it once they pass."
    fi
    echo
    echo "- Commit of main: \`${cand:0:12}\`"
    echo "- Catalog: $CATALOG"
    echo "- Started by: $WHY"
    [ -z "${ISSUE:-}" ] || echo "- Production issue: #$ISSUE"
    echo
    echo "Commits ($COUNT):"
    echo
    echo '~~~'
    git log --format='%h %s' "$current..$cand" | head -n "$LISTED_COMMITS" || true
    [ "$COUNT" -le "$LISTED_COMMITS" ] || echo "... and $((COUNT - LISTED_COMMITS)) more"
    echo '~~~'
  } > "$body"

  ensure_label
  push_ref "$cand:refs/heads/$BRANCH"
  [ -z "${TESTED:-}" ] || opener=gh
  url=$(try "$opener" pr create --base production --head "$BRANCH" --label "$RELEASE_LABEL" \
    --label "$HUMAN_LABEL" --assignee "$ASSIGNEES" \
    --title "Promote main to production: $COUNT commit(s) up to ${cand:0:7}" --body-file "$body")
  rm -f "$body"
  PR_NUMBER=${url##*/}
  RESULT=opened
  echo "Opened #$PR_NUMBER for ${cand:0:12}; the super run tests it."
  printf '### Promotion of `%s` opened (%s)\n\nPull request #%s, catalog %s, %s commits.\n' \
    "${cand:0:12}" "$WHY" "$PR_NUMBER" "$CATALOG" "$COUNT" >> "$SUMMARY"
}

# Merges promotion pull request $1, whose head is branch $2 at commit $3, into production. Sets RESULT
# to merged, and PR_NUMBER, COUNT and CATALOG.
merge_pr() {
  local num=$1 head=$2 cand=$3 current said attempt
  same_history "$cand"
  current=$(git rev-parse origin/production)
  COUNT=$(git rev-list --count "$current..$cand") CATALOG=$(catalog "$cand")
  # GitHub works out whether a new pull request merges cleanly in the background, so the first
  # attempt right after `create` can be refused for a moment.
  said=$(mktemp)
  for attempt in $(seq 1 "$MERGE_ATTEMPTS"); do
    if promoter pr merge "$num" --merge --match-head-commit "$cand" 2>"$said"; then break; fi
    [ "$attempt" -lt "$MERGE_ATTEMPTS" ] || fail "Pull request #$num could not be merged into production; it stays open. GitHub said: $(tr '\n' ' ' <"$said" | cut -c1-600)"
    sleep "$MERGE_RETRY_SECONDS"
  done
  rm -f "$said"
  promoter api -X DELETE "repos/$REPO/git/refs/heads/$head" --silent || true

  git fetch -q origin production
  git diff --quiet "$cand" origin/production \
    || fail "production's files differ from ${cand:0:12} after merging #$num. Someone changed production meanwhile; see docs/deploy-cloudflare.md, 'production has diverged'."

  RESULT=merged PR_NUMBER=$num
  moved
  echo "Merged #$num: production now has ${cand:0:12} ($COUNT commits)."
  printf '### production <- `%s` (%s)\n\nPull request #%s, catalog %s, %s commits.\n' \
    "${cand:0:12}" "$WHY" "$num" "$CATALOG" "$COUNT" >> "$SUMMARY"
}

# The emergency way: merges at once, without the super run, as $1 asked for reason $2. The candidate
# is commit $3 of main when one is named, else the open promotion pull request's, else main's newest
# green commit. Sets RESULT to merged, created, uptodate or waiting, or returns 1 with WHY_NOT.
skip_super() {
  local who=$1 reason=$2 sha=$3 p='' num head
  [ -n "$sha" ] || p=$(pending)
  if [ -n "$p" ]; then
    read -r num head CAND <<<"$p"
  else
    vet "$sha" || return 1
    open_candidate
    case $RESULT in
      uptodate | waiting) return 0 ;;
      noproduction)
        push_ref "$CAND:refs/heads/production"
        RESULT=created CATALOG=$(catalog "$CAND")
        moved
        echo "production did not exist; it is now ${CAND:0:12}."
        return 0
        ;;
    esac
    num=$PR_NUMBER head=$BRANCH
  fi
  try gh pr comment "$num" --body "@$who merges this without the super run, the emergency way (\`/skip-super\`): $reason"
  merge_pr "$num" "$head" "$CAND"
}

# --- the daily promotion --------------------------------------------------------------------------

# The newest super run on MAIN_BRANCH that passed every job, as "id sha url", or nothing. A run on
# a promotion pull request is on its promote/ branch, never main's.
newest_main_pass() {
  try gh run list --workflow "$SUPER_WORKFLOW" --branch "$MAIN_BRANCH" --status success --limit 1 \
    --json databaseId,headSha,url --jq '.[0] // empty | "\(.databaseId) \(.headSha) \(.url)"'
}

# Promotes the commit the newest passing super run on main tested, once a run (`daily`): it opens
# the promotion pull request without a second super run and merges it. An open promotion already
# carrying the commit is merged when it is that commit and left to its own run when it is newer; one
# of an older commit is superseded.
daily_promotion() {
  local line id sha url num head psha p
  line=$(newest_main_pass)
  [ -n "$line" ] || return 0
  read -r id sha url <<<"$line"
  [ "$id" -gt "${daily//-/0}" ] || return 0
  daily=$id
  # A run on a commit production already has (the newest one that passed may be days old) is done
  # with, quietly: vetting it could only say why it is not promoted, about a commit that is live. The
  # first run after #2959 said so of 2026-10-29's run on 52bd29e11fb2, whose CI had aged out.
  if has_production && git merge-base --is-ancestor "$sha" origin/production 2>/dev/null; then return 0; fi
  WHY="the daily promotion"
  if ! vet "$sha"; then
    say "The super run on \`$MAIN_BRANCH\` passed on \`${sha:0:12}\` ([run]($url)), but it is not promoted: $WHY_NOT"
    return 0
  fi
  p=$(pending)
  if [ -n "$p" ]; then
    read -r num head psha <<<"$p"
    if [ "$psha" != "$CAND" ] && git merge-base --is-ancestor "$CAND" "$psha"; then
      echo "#$num already carries ${CAND:0:12}; its own super run decides it."
      return 0
    fi
  fi
  TESTED="[run]($url)" open_candidate
  case $RESULT in
    opened | testing)
      merge_pr "$PR_NUMBER" "$BRANCH" "$CAND"
      say "Merged to production in #$PR_NUMBER, the daily promotion: the super run on \`$MAIN_BRANCH\` passed every job on \`${CAND:0:12}\` ([run]($url)). $COUNT commit(s), catalog $CATALOG; the deploy follows. $(run_link)"
      ;;
    waiting) daily=$((id - 1)) ;;
    noproduction) say "The super run on \`$MAIN_BRANCH\` passed on \`${CAND:0:12}\` ([run]($url)), but \`production\` does not exist yet: \`/skip-super\` with a reason creates it." ;;
  esac
}

# --- reconciling with the super run --------------------------------------------------------------

# Where the open promotion stands, from its newest super run on its head commit: STATUS is idle (none
# open), testing or failed, with PR_NUMBER, CAND and SUPER_URL. A run that passed merges the pull
# request (STATUS idle again); one that failed is said on the issue once a run attempt.
reconcile() {
  local p num head sha line id attempt status conclusion
  STATUS=idle SUPER_URL=''
  p=$(pending)
  [ -n "$p" ] || return 0
  read -r num head sha <<<"$p"
  PR_NUMBER=$num CAND=$sha STATUS=testing
  line=$(try gh run list --workflow "$SUPER_WORKFLOW" --branch "$head" --event pull_request --commit "$sha" \
    --limit 1 --json databaseId,attempt,status,conclusion,url \
    --jq '.[0] // empty | "\(.databaseId) \(.attempt) \(.status) \(.conclusion) \(.url)"')
  [ -n "$line" ] || return 0
  read -r id attempt status conclusion SUPER_URL <<<"$line"
  [ "$status" = completed ] || return 0
  if [ "$conclusion" = success ]; then
    WHY="the super run passed"
    merge_pr "$num" "$head" "$sha"
    STATUS=idle
    say "Merged to production in #$num: the super run passed every job on \`${sha:0:12}\` ([run]($SUPER_URL)). $COUNT commit(s), catalog $CATALOG; the deploy follows. $(run_link)"
    return 0
  fi
  STATUS=failed
  [ "$reported" != "$id.$attempt" ] || return 0
  reported="$id.$attempt"
  say "The super run ended \`$conclusion\` on \`${sha:0:12}\` ([run]($SUPER_URL)), so #$num does not merge. Its report hands the fix to the night bot as \`priority:high\` (the issue \"$SUPER_TITLE\"). #$num stays open: re-running the run's failed jobs merges it once they pass, a \`/promote\` once the fix is on main tests the newer commit instead, and \`/skip-super\` with a reason merges it now."
}

# --- the queue -------------------------------------------------------------------------------

# "#1533 Patch: the stats drill-down's card ...", from a squash commit's "(#1533)" or a merge
# commit's "Merge pull request #1533"; any other commit by its short sha. Escaped for a table cell.
row_label() {
  local s
  s=$(git log -1 --format=%s "$1")
  if [[ $s =~ ^(.*)\ \(#([0-9]+)\)$ ]]; then
    s="#${BASH_REMATCH[2]} ${BASH_REMATCH[1]}"
  elif [[ $s =~ ^Merge\ pull\ request\ \#([0-9]+)\  ]]; then
    s="#${BASH_REMATCH[1]} $(git log -1 --format=%b "$1" | awk 'NF { print; exit }' || true)"
  else
    s="\`$(git rev-parse --short "$1")\` $s"
  fi
  echo "${s//|/\\|}"
}

# Commit $1's push-to-main CI, from RUNS (a "sha status conclusion" line a run).
ci_cell() {
  local st
  st=$(grep -m1 "^$1 " <<<"$RUNS" | cut -d' ' -f2,3 || true)
  case $st in
    "completed success") echo "✅ passed" ;;
    "completed failure" | "completed timed_out" | "completed startup_failure") echo "❌ failed" ;;
    completed*) echo "${st#completed }" ;;
    "") echo "—" ;;
    *) echo "⏳ running" ;;
  esac
}

# QUEUE, the body's queue, and WAITING, its length: what main has that production does not, newest
# first, and where each commit stands.
refresh_queue() {
  local prod green c n=0 more=0 rows='' cell
  prod=$(git rev-parse -q --verify origin/production || true)
  green=$(newest_green || true)
  RUNS=$(gh run list --workflow ci.yml --branch main --event push --limit 200 --json headSha,status,conclusion \
    --jq '.[] | "\(.headSha) \(.status) \(.conclusion)"' 2>/dev/null || true)
  while read -r c; do
    if [ -n "$prod" ] && git merge-base --is-ancestor "$c" "$prod"; then break; fi
    if [ "$n" -ge "$LISTED_COMMITS" ]; then more=1; break; fi
    n=$((n + 1))
    if [ "$STATUS" != idle ] && git merge-base --is-ancestor "$c" "$CAND"; then cell="in #$PR_NUMBER"
    elif [ -n "$green" ] && git merge-base --is-ancestor "$c" "$green"; then cell="next /promote"
    else cell="waits for CI"; fi
    rows+="| $(row_label "$c") | $(ci_cell "$c") | $cell |"$'\n'
  done < <(git rev-list --first-parent origin/main)
  WAITING=$n

  QUEUE=$'\n### Queue\n\n'
  if [ -n "$prod" ]; then
    QUEUE+="Production serves \`main\` as of \`$(git rev-parse --short "$(git rev-parse -q --verify "$prod^2" || echo "$prod")")\`."
  else
    QUEUE+="Production serves nothing yet."
  fi
  if [ "$n" = 0 ]; then
    QUEUE+=$' Everything on `main` is live.\n'
    return 0
  fi
  QUEUE+=" Not live yet, newest first: $n"
  [ "$more" = 0 ] || QUEUE+=" (and older)"
  QUEUE+=$'. `/promote` takes `main` up to its newest commit whose CI passed.\n\n'
  QUEUE+=$'| Pull request | CI | Production |\n| --- | --- | --- |\n'
  QUEUE+=$rows
}

# --- the production issue ----------------------------------------------------------------------

ensure_label() {
  gh label create "$RELEASE_LABEL" --color 5319e7 --force \
    --description "The merge of main into production (promote-production.yml)" >/dev/null
}
open_issues() {
  gh issue list --state open --label "$RELEASE_LABEL" --limit 100 --json number --jq 'map(.number) | sort | .[]'
}

title_for() {
  case $STATUS in
    testing) echo "Production: the super run is testing ${CAND:0:7} (#$PR_NUMBER)" ;;
    failed) echo "Production: the super run failed on ${CAND:0:7} (#$PR_NUMBER)" ;;
    *)
      # No count: the body's queue has it. A title with the count changed with every commit of main,
      # and GitHub keeps every retitle in the issue's timeline.
      if [ "$WAITING" = 0 ]; then echo "Production: everything on main is live"
      else echo "Production: commits on main are not live yet"; fi
      ;;
  esac
}

body_for() {
  printf '<!-- promote-production last=%s reported=%s daily=%s -->\n' "$last" "$reported" "$daily"
  case $STATUS in
    testing)
      printf '**Production:** #%s promotes `main` up to `%s`, and the super run is testing it%s. It merges and deploys once every job passes.\n' \
        "$PR_NUMBER" "${CAND:0:12}" "${SUPER_URL:+ ([run]($SUPER_URL))}"
      ;;
    failed)
      printf '**Production:** the super run failed on `%s` ([run](%s)), so #%s waits. Its fix is the night bot'"'"'s ("%s"). Re-running the failed jobs merges #%s once they pass, and the next daily super run on `main` that passes, or a `/promote` after the fix, takes the newer commit instead.\n' \
        "${CAND:0:12}" "$SUPER_URL" "$PR_NUMBER" "$SUPER_TITLE" "$PR_NUMBER"
      ;;
    *)
      printf '**Production** promotes itself once a day: when the daily super run on `main` passes every job, the commit it tested merges into `production` and deploys. A red run is the night bot'"'"'s to fix, and production waits for the next one that passes. `/promote` here goes sooner: the super run tests a pull request of `main`'"'"'s newest green commit, and it merges and deploys once every job passes.\n'
      ;;
  esac
  printf '\n**Staging** deploys every commit of `main` whose CI passed, by itself (`deploy-staging.yml`): nothing here holds it.\n'
  printf '%s' "$QUEUE"
  cat <<'TEXT'

### Commands

Comment here. Only people with write access are heard; a command gets a 👍, or a 😕 when it was refused, and a reply.

| Comment | Effect |
| --- | --- |
| `/promote` | Sooner than the daily promotion: opens a pull request of `main`'s newest commit whose CI passed into `production`. The super run (`super.yml`) tests it, and it merges and deploys once every job passes. A red run hands its fix to the night bot as `priority:high`, and the pull request waits. |
| `/promote <sha>` | The same, for that commit of `main`. |
| `/skip-super <reason>` | The emergency way: merges the open pull request, or `main`'s newest green commit, now, without the super run. The reason is required, and it is recorded here and on the pull request with who and which commit. |
| `/cancel` | Closes the open pull request. Nothing merges. |

A newer `/promote`, or the daily promotion of a newer commit, closes the open pull request as superseded. Running *promote production* from the Actions tab does the same as `/promote`, or as `/skip-super` with its reason box filled. A promotion or a deploy that fails opens "CI: production promotion failed" for Squishy (`promote-production.yml`'s `report`).
TEXT
  printf '\nA fresh issue takes over from this one every %s days, carrying its state, so the thread stays short: [the closed ones](%s) are the history. Plan: `docs/deploy-cloudflare.md`. Written by `scripts/promote-production.sh`; the first line is its state, so leave it be.\n' \
    "$ROTATE_DAYS" "${GITHUB_SERVER_URL:-https://github.com}/$REPO/issues?q=is%3Aissue+is%3Aclosed+label%3A%22${RELEASE_LABEL// /+}%22"
}

# Reads issue $1, or starts its state afresh when it has lost its state line. A line the countdown
# wrote before 2026-11-02 (`due=… held=… last=…`) keeps its `last`, and one from before the daily
# promotion (no `daily`) starts it at none: production already holding the commit a passing run
# tested makes that run a no-op.
load_issue() {
  local m info
  info=$(try gh issue view "$1" --json title,body,createdAt)
  title=$(jq -r .title <<<"$info")
  body=$(jq -r .body <<<"$info" | tr -d '\r')
  created=$(jq -r '.createdAt // empty | fromdateiso8601' <<<"$info" 2>/dev/null) \
    || echo "::warning::#$1's createdAt did not parse, so it is not handed over to a fresh issue."
  m=$(grep -o 'promote-production [^>]*' <<<"$body" | head -n1 || true)
  last=0 reported=- daily=-
  if [[ $m =~ last=([0-9]+) ]]; then last=${BASH_REMATCH[1]}; else echo "#$1 has lost its state line; starting it afresh."; fi
  if [[ $m =~ reported=([0-9.]+) ]]; then reported=${BASH_REMATCH[1]}; fi
  if [[ $m =~ daily=([0-9]+) ]]; then daily=${BASH_REMATCH[1]}; fi
}
save_issue() {
  local f want text
  want=$(title_for)
  text=$(body_for)
  if [ "$text" != "$body" ] || [ "$want" != "$title" ]; then
    f=$(mktemp)
    printf '%s\n' "$text" > "$f"
    try gh issue edit "$ISSUE" --title "$want" --body-file "$f"
    rm -f "$f"
  fi
  title=$want body=$text
}
# Opens a production issue holding the state in hand (last, reported, daily), and makes it ISSUE.
new_issue() {
  local f url
  title=$(title_for) body=$(body_for) created=$NOW
  f=$(mktemp)
  printf '%s\n' "$body" > "$f"
  ensure_label
  url=$(try gh issue create --title "$title" --label "$RELEASE_LABEL" --label "$HUMAN_LABEL" \
    --assignee "$ASSIGNEES" --body-file "$f")
  rm -f "$f"
  ISSUE=${url##*/}
}
# The production issue, ISSUE: the oldest open one, a newer duplicate closed; or a new one when none
# is open.
find_issue() {
  local issues n
  issues=$(open_issues)
  if [ -z "$issues" ]; then
    last=0 reported=- daily=-
    new_issue
    echo "Opened #$ISSUE, the production issue."
  else
    ISSUE=$(head -n1 <<<"$issues")
    for n in $issues; do
      [ "$n" = "$ISSUE" ] || gh issue close "$n" --reason "not planned" --comment "Superseded by #$ISSUE, the production issue that stays open."
    done
  fi
  load_issue "$ISSUE"
}
# Once ISSUE is ROTATE_DAYS old, a fresh one takes over with its state line (comment ids are the
# repository's, so `last` holds across issues), and the old one is retitled with the days it covered
# and closed with a link to the new one. Run after this run's replies, which stay with the comments
# they answer.
rotate_issue() {
  local old=$ISSUE span
  [ -n "$created" ] && [ $((NOW - created)) -ge $((ROTATE_DAYS * 86400)) ] || return 0
  span="$(jq -rn --argjson t "$created" '$t | strftime("%Y-%m-%d")') to $(jq -rn --argjson t "$NOW" '$t | strftime("%Y-%m-%d")')"
  new_issue
  try gh issue close "$old" --reason completed \
    --comment "Continued in #$ISSUE: a fresh production issue takes over every $ROTATE_DAYS days, with this one's state, so no thread grows long. Comment there from now on."
  try gh issue edit "$old" --title "Production, $span (continued in #$ISSUE)"
  echo "Opened #$ISSUE, the production issue from now on; closed #$old ($span)."
}
say() { try gh issue comment "$ISSUE" --body "$1" >/dev/null; }
react() { gh api -X POST "repos/$REPO/issues/comments/$1/reactions" -f content="$2" --silent >/dev/null || true; }

# --- the commands --------------------------------------------------------------------------------

# Says on the issue what came of a promotion's RESULT, asked by $1.
said_promote() {
  case $RESULT in
    opened) say "Opened #$PR_NUMBER ($1): $COUNT commit(s) of \`main\` up to \`${CAND:0:12}\`, catalog $CATALOG. The super run tests it now, and it merges and deploys once every job passes." ;;
    testing) say "#$PR_NUMBER already promotes \`${CAND:0:12}\`; the super run is on it." ;;
    uptodate) say "Nothing to merge: \`production\` already has \`${CAND:0:12}\`." ;;
    waiting) say "Not promoting \`${CAND:0:12}\` yet: main's newest commit changes \`.github/workflows\` and its CI has not passed, and GitHub refuses this workflow's token that push. Ask again once CI on main is green." ;;
    noproduction) say "\`production\` does not exist yet, so a pull request has nothing to merge into: \`/skip-super\` with a reason creates it at \`${CAND:0:12}\`." ;;
  esac
}
# The same for /skip-super, asked by $1 for reason $2.
said_skip() {
  case $RESULT in
    merged) say "Merged #$PR_NUMBER into production without the super run, at @$1's word: $2 ($COUNT commit(s) up to \`${CAND:0:12}\`, catalog $CATALOG). The deploy follows. $(run_link)" ;;
    created) say "Created \`production\` at \`${CAND:0:12}\` without the super run, at @$1's word: $2. The deploy follows." ;;
    *) said_promote "@$1" ;;
  esac
}

# Reads the comments on the issue newer than `last` and carries out each command, in order. When the
# comment that woke this run is on a production issue that has since closed (COMMENT_ISSUE), that
# one's comments are read too, so a command left there is not lost; the replies come here.
apply_commands() {
  local line id who cmdline cmd arg sha p num head from=$ISSUE
  [ -z "$COMMENT_ISSUE" ] || [ "$COMMENT_ISSUE" = "$ISSUE" ] || from="$ISSUE $COMMENT_ISSUE"
  while IFS= read -r line; do
    id=$(jq -r .id <<<"$line")
    [ "$id" -gt "$last" ] || continue
    last=$id
    cmdline=$(jq -r .body <<<"$line" | tr -d '\r' \
      | grep -iE -m1 '^[[:space:]]*/(promote|fast-forward|skip-super|cancel|hold|resume|release|unhold|delay)([[:space:]]|$)' || true)
    [ -n "$cmdline" ] || continue
    read -r cmd arg <<<"$cmdline"
    cmd=$(tr '[:upper:]' '[:lower:]' <<<"${cmd#/}")
    who=$(jq -r .login <<<"$line")
    if ! jq -e '(.assoc == "OWNER" or .assoc == "MEMBER" or .assoc == "COLLABORATOR") and (.bot | not)' <<<"$line" >/dev/null; then
      echo "Comment $id: /$cmd from $who ignored: not someone with write access."
      react "$id" confused
      continue
    fi
    case $cmd in
      promote | fast-forward)
        sha=''
        if [[ ${arg%% *} =~ ^[0-9a-fA-F]{7,40}$ ]]; then sha=${arg%% *}; fi
        WHY="/$cmd from @$who"
        if ! vet "$sha"; then react "$id" confused; say "$WHY_NOT"; continue; fi
        open_candidate
        said_promote "@$who"
        ;;
      skip-super)
        if [ -z "${arg// /}" ]; then
          react "$id" confused
          say "\`/skip-super\` needs a reason after it, such as \`/skip-super the fuzz wave is red on a seed this change does not touch\`: the reason is recorded with the merge."
          continue
        fi
        WHY="/skip-super from @$who: $arg"
        if ! skip_super "$who" "$arg" ""; then react "$id" confused; say "$WHY_NOT"; continue; fi
        said_skip "$who" "$arg"
        ;;
      cancel)
        p=$(pending)
        if [ -z "$p" ]; then
          say "No promotion pull request is open, so there is nothing to cancel."
        else
          read -r num head _ <<<"$p"
          try promoter pr close "$num" --comment "Cancelled by @$who on #$ISSUE."
          promoter api -X DELETE "repos/$REPO/git/refs/heads/$head" --silent || true
          say "Closed #$num, as @$who asked. Nothing merges."
        fi
        ;;
      *)
        react "$id" confused
        say "There is nothing to hold, resume or delay: production follows the super run, promoting itself when the daily run on \`$MAIN_BRANCH\` passes, or a \`/promote\` here once its own run passes. \`/cancel\` closes an open promotion pull request."
        continue
        ;;
    esac
    react "$id" '+1'
  done < <(for n in $from; do
      gh api "repos/$REPO/issues/$n/comments" --paginate \
        --jq '.[] | {id, login: .user.login, bot: (.user.type == "Bot"), assoc: .author_association, at: .created_at, body} | tojson'
    done | jq -c -s 'sort_by(.id) | .[]')
}

# A manual run: /promote for INPUT_SHA, or /skip-super with INPUT_SKIP_REASON.
dispatch() {
  local who=${ACTOR:-someone}
  if [ -n "${INPUT_SKIP_REASON// /}" ]; then
    WHY="a manual run by @$who, without the super run: $INPUT_SKIP_REASON"
    skip_super "$who" "$INPUT_SKIP_REASON" "$INPUT_SHA" || fail "$WHY_NOT"
    [ "$RESULT" != waiting ] || fail "Nothing merged: main's newest commit changes .github/workflows and its CI has not passed yet, and GitHub refuses this workflow's token a push of workflow files that differ from main's. Run this again once CI on main is green."
    said_skip "$who" "$INPUT_SKIP_REASON"
    return 0
  fi
  WHY="a manual run by @$who"
  vet "$INPUT_SHA" || fail "$WHY_NOT"
  open_candidate
  case $RESULT in
    waiting) fail "Nothing opened: main's newest commit changes .github/workflows and its CI has not passed yet, and GitHub refuses this workflow's token a push of workflow files that differ from main's. Run this again once CI on main is green." ;;
    noproduction) fail "production does not exist yet: run this again with a skip_super_reason, which creates it." ;;
    uptodate) ;;
    *) said_promote "by hand, @$who" ;;
  esac
}

# --- the events ------------------------------------------------------------------------------

case $EVENT in
  issue_comment | workflow_run | workflow_dispatch) ;;
  *) fail "promote-production.sh does not handle the $EVENT event: production follows a passing super run, by itself or on /promote." ;;
esac

QUEUE='' STATUS=idle WAITING=0 WHY='' RESULT='' PR_NUMBER='' BRANCH='' CAND='' SUPER_URL='' COUNT=0 CATALOG='' created=''
find_issue
case $EVENT in
  issue_comment) apply_commands ;;
  workflow_dispatch) dispatch ;;
esac
daily_promotion
reconcile
refresh_queue
rotate_issue
save_issue
