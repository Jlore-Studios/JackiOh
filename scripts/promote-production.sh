#!/usr/bin/env bash
# Merges main into production through a pull request, and keeps the one issue that counts down to the
# next merge and shows the queue. .github/workflows/promote-production.yml runs it, and its `deploy`
# job then puts what production holds on jackioh.com: the site and its game server, together
# (wrangler.jsonc). docs/deploy-cloudflare.md is the plan and the runbook.
# apps/web/src/net/promote-production.test.ts holds it in place, against a throwaway repository and a
# stand-in for `gh`.
#
# Staging is not this script's: deploy-staging.yml deploys every commit of main whose push-to-main CI
# passed, by itself, and nothing here holds or hurries it.
#
# The countdown issue. One issue labeled RELEASE_LABEL stays open for good. It says "Merging to
# production in N hours", lists what main has that is not live yet (the queue, below), and carries,
# in a hidden comment, the time (`due`), whether it is on hold (`held`) and the last comment it has
# read (`last`). When a merge goes out it says so in a comment and counts down to the next; it is
# never closed and opened again. Anyone with write access comments on it:
#   /hold [why]            nothing merges until /resume
#   /resume                lifts the hold; a merge already overdue goes out at the next check
#   /delay <n>[h|d]        moves the merge n hours (or days) later, at most MAX_DELAY_HOURS a time
#   /fast-forward          merges now, past a hold
# A first word `production` (or `prod`, `cloudflare`) is allowed and changes nothing. One that names
# staging (`staging`, or `vercel`, which was staging once) is refused: staging is not held from
# here, and read as the reason it would hold production instead.
# Every check also brings the title's "in N hours" up to date, so it counts down hour by hour.
# Each command gets a +1 reaction, or a confused one when it was refused (not from a collaborator, or
# not understood). The next merge time is the first RELEASE_HOUR in RELEASE_TZ (10:00 AM Central,
# through daylight saving) at least MIN_GAP_HOURS ahead, and every time it shows is in that zone ("CT").
#
# The queue. The issue's body lists every commit of main (by its pull request) that is not live in
# production, newest first, with its push-to-main CI and where it stands: the next merge (production
# takes main up to its newest commit whose CI passed), waiting for CI, or held.
#
# A catalog bump waits for the countdown like any other change. Production's site and server ship
# together from production, so the bundle and the server always agree on the catalog version (SPEC
# §9.4, R105); when Render deployed the server from main on every push, a bump went out at once to
# keep the two together, and that fast path is gone with it.
#
# What runs it, and what each does (EVENT is github.event_name):
#   schedule, issue_comment, workflow_run
#                            a check: read the new commands, merge when due and not on hold, say so on
#                            the issue and count down to the next. workflow_run is a green CI run on
#                            main; it is here because GitHub has not started this repository's scheduled
#                            runs (CLAUDE.md, the night bot), so the first green push after the issue's
#                            time is what merges, and the cron is a second chance, not the mechanism.
#   workflow_dispatch        by hand: merges the newest green commit of main, or INPUT_SHA, at once,
#                            hold or no hold, and when it merged something the countdown starts over.
#
# The merge. The candidate is a commit of main whose push-to-main CI run passed (the newest, unless
# one is named). It is pushed as promote/<date>-<sha>, a pull request into production is opened with
# the commits listed, and it is merged with a merge commit. Because production only ever receives
# merges of green main commits, its files always equal the candidate's; a production that carries
# anything else (someone committed to it) stops the run before anything is opened, and so does a
# result that differs from the candidate. Nothing is ever force-pushed. If production does not exist
# the candidate is pushed as production, with no pull request.
#
# When production moves it writes `promoted=true` to GITHUB_OUTPUT, which is what starts the workflow's
# `deploy` job (GitHub Actions builds production, the bundle and the server's image, and deploys both
# with wrangler; Cloudflare builds nothing).
#
# It talks to GitHub through `gh` and to git through the checkout's own credentials: the workflow
# checks out main with its whole history. Two tokens. What moves production (the push of the
# promote branch, its pull request, the merge, the branch's deletion) goes out as PROMOTE_TOKEN, and
# the checkout pushes with the same one: whenever main changed a file under .github/workflows since
# production, GitHub refuses that ref update from the Actions token ("refusing to allow a GitHub App
# to create or update workflow ... without `workflows` permission"), and no `permissions:` entry can
# grant it. The workflow passes BOT_GITHUB_TOKEN, which has the `workflow` scope (bot/README.md).
# Everything else (the countdown issue, its comments, reactions and the label) stays on GH_TOKEN, the
# Actions token, whose writes start no workflow, so the night bot never sees the issue. The bot's
# pull request starts no loop either: CI runs on it as on any pull request, but a pull request's CI
# never starts promote-production.yml or deploy-staging.yml (only a push to main's does), the push of
# a promote branch or of production starts nothing (every workflow's `push` names branches), and the
# bot's harness ignores its own events. Needs GNU or BSD coreutils only for `date +%s`; the clock
# arithmetic is plain seconds and jq does the formatting.

# The backticks in the printf formats below are Markdown for GitHub, not command substitutions.
# shellcheck disable=SC2016
set -Eeuo pipefail
# A command that fails on its own (not through `fail`) still leaves an annotation saying which.
trap 'echo "::error::scripts/promote-production.sh line $LINENO: \`$BASH_COMMAND\` failed (exit $?)" >&2' ERR

: "${GITHUB_REPOSITORY:?}"
: "${EVENT:?}"
: "${RELEASE_LABEL:=production merge}"   # keep equal to the label named in promote-production.yml's `if`
# Who the countdown issues and the promotion pull requests are assigned to, and the label that keeps
# the night bot off them (docs/issues-and-patches.md: people's work is `human`, for both of them).
: "${ASSIGNEES:=jgoetzmann,MaxGoetzmann}"
: "${HUMAN_LABEL:=human}"
: "${RELEASE_TZ:=America/Chicago}"
: "${RELEASE_ZONE_NAME:=CT}"
: "${RELEASE_HOUR:=10}"
: "${MIN_GAP_HOURS:=12}"
: "${MAX_DELAY_HOURS:=168}"
: "${LISTED_COMMITS:=100}"
: "${MERGE_ATTEMPTS:=4}"
: "${MERGE_RETRY_SECONDS:=5}"
: "${PROMOTE_TOKEN:=${GH_TOKEN:-}}"
# The patch history whose newest entry names the catalog version a commit ships (crates/cards/build.rs
# compiles the same entry into the server and the bundle).
: "${PATCHES_JSON:=crates/cards/patches/patches.json}"
REPO=$GITHUB_REPOSITORY
SUMMARY=${GITHUB_STEP_SUMMARY:-/dev/null}

now() { echo "${NOW:-$(date -u +%s)}"; }
# RELEASE_TZ's offset from UTC at epoch $1, in seconds (-18000 for CDT). jq's localtime converts
# correctly but labels the zone wrongly, so the offset is worked out rather than printed.
offset() { TZ=$RELEASE_TZ jq -nr --argjson t "$1" '($t | localtime | mktime) - ($t | floor)'; }
# "2026-10-05 10:00 AM CT"
stamp() {
  TZ=$RELEASE_TZ jq -nr --argjson t "$1" --arg z "$RELEASE_ZONE_NAME" '$t | strflocaltime("%Y-%m-%d %I:%M %p ") + $z' \
    | sed -E 's/ 0([1-9]:)/ \1/'
}
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

newest_green() {
  gh run list --workflow ci.yml --branch main --event push --status success --limit 1 \
    --json headSha --jq '.[0].headSha // ""'
}
# Prints the full sha of a commit of main whose push-to-main CI run passed: $1, or the newest.
vet() {
  local c=$1
  [ -n "$c" ] || c=$(newest_green)
  [ -n "$c" ] || fail "No push to main has a passing CI run; nothing can be promoted."
  c=$(git rev-parse --verify -q "$c^{commit}") || fail "${1:-$c} is not a commit."
  git merge-base --is-ancestor "$c" origin/main || fail "$c is not on main."
  [ -n "$(gh run list --workflow ci.yml --branch main --event push --status success \
    --commit "$c" --limit 1 --json headSha --jq '.[].headSha')" ] \
    || fail "$c has no passing push-to-main CI run."
  echo "$c"
}

# Only when production is moved with the Actions token (no BOT_GITHUB_TOKEN secret): GitHub refuses
# that token a push whose .github/workflows differ from the default branch's, which is the case while
# main's newest commit changes a workflow and its CI is still running, so the newest green commit is
# older. The push of promote/<sha> was refused on 2026-10-04 for exactly that. True (and
# RESULT=waiting) when commit $1 is such a commit: wait for that CI run, whose workflow_run then
# merges main's newest commit, whose workflows are main's. The bot's token has the workflow scope
# and never waits.
must_wait() {
  [ "$PROMOTE_TOKEN" = "${GH_TOKEN:-}" ] || return 1
  git diff --quiet "$1" origin/main -- .github/workflows && return 1
  RESULT=waiting
  echo "::notice::Not merging ${1:0:12} yet: main has changed .github/workflows since it (newest $(git rev-parse --short origin/main)), and GitHub refuses this token a push of workflow files that differ from main's. When CI passes on main's newest commit, that run merges it."
}

# Merges commit $1 of main into production. $2 says why (for the pull request and the issue's
# comment). Sets RESULT to merged, created, uptodate or waiting, and for a merge PR_NUMBER, COUNT, CANDIDATE,
# CATALOG and WHY.
promote() {
  local cand=$1 current base branch pr url body attempt said
  WHY=$2 PR_NUMBER='' COUNT=0 CANDIDATE=$cand CATALOG=$(catalog "$cand")

  if ! has_production; then
    must_wait "$cand" && return 0
    push_ref "$cand:refs/heads/production"
    RESULT=created
    moved
    echo "production did not exist; it is now ${cand:0:12}."
    printf '### production created at `%s` (%s)\n\nCatalog: %s\n' "${cand:0:12}" "$WHY" "$CATALOG" >> "$SUMMARY"
    return 0
  fi
  current=$(git rev-parse origin/production)
  if git merge-base --is-ancestor "$cand" "$current"; then
    RESULT=uptodate
    echo "production already has ${cand:0:12}; nothing to merge."
    return 0
  fi
  must_wait "$cand" && return 0
  base=$(git merge-base "$current" "$cand") || fail "production and $cand share no history."
  git diff --quiet "$base" "$current" || fail "production has diverged: it carries changes that are not on main (git diff ${base:0:12} ${current:0:12}). Not merging; see docs/deploy-cloudflare.md, 'production has diverged'."

  COUNT=$(git rev-list --count "$current..$cand")
  branch="promote/$(date -u +%Y%m%d)-${cand:0:7}"
  body=$(mktemp)
  {
    echo "Merges \`main\` into \`production\`, which this workflow then builds and deploys: the site and its server, together."
    echo
    echo "- Newest green commit of main: \`${cand:0:12}\`"
    echo "- Catalog: $CATALOG"
    echo "- Started by: $WHY"
    [ -z "${3:-}" ] || echo "- Countdown issue: #$3"
    echo
    echo "Commits ($COUNT):"
    echo
    echo '~~~'
    git log --format='%h %s' "$current..$cand" | head -n "$LISTED_COMMITS" || true
    [ "$COUNT" -le "$LISTED_COMMITS" ] || echo "... and $((COUNT - LISTED_COMMITS)) more"
    echo '~~~'
  } > "$body"

  ensure_label
  push_ref "$cand:refs/heads/$branch"
  pr=$(try promoter pr list --base production --head "$branch" --state open --json number --jq '.[0].number // empty')
  if [ -z "$pr" ]; then
    url=$(try promoter pr create --base production --head "$branch" --label "$RELEASE_LABEL" \
      --label "$HUMAN_LABEL" --assignee "$ASSIGNEES" \
      --title "Promote main to production: $COUNT commit(s) up to ${cand:0:7}" --body-file "$body")
    pr=${url##*/}
  fi
  rm -f "$body"

  # GitHub works out whether a new pull request merges cleanly in the background, so the first
  # attempt right after `create` can be refused for a moment.
  said=$(mktemp)
  for attempt in $(seq 1 "$MERGE_ATTEMPTS"); do
    if promoter pr merge "$pr" --merge --match-head-commit "$cand" 2>"$said"; then break; fi
    [ "$attempt" -lt "$MERGE_ATTEMPTS" ] || fail "Pull request #$pr could not be merged into production; it stays open. GitHub said: $(tr '\n' ' ' <"$said" | cut -c1-600)"
    sleep "$MERGE_RETRY_SECONDS"
  done
  rm -f "$said"
  promoter api -X DELETE "repos/$REPO/git/refs/heads/$branch" --silent || true

  git fetch -q origin production
  git diff --quiet "$cand" origin/production \
    || fail "production's files differ from ${cand:0:12} after merging #$pr. Someone changed production meanwhile; see docs/deploy-cloudflare.md, 'production has diverged'."

  RESULT=merged PR_NUMBER=$pr
  moved
  echo "Merged #$pr: production now has ${cand:0:12} ($COUNT commits)."
  printf '### production <- `%s` (%s)\n\nPull request #%s, catalog %s, %s commits.\n' \
    "${cand:0:12}" "$WHY" "$pr" "$CATALOG" "$COUNT" >> "$SUMMARY"
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

# QUEUE, the body's queue: what main has that production does not, newest first.
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
    if [ -n "$green" ] && git merge-base --is-ancestor "$c" "$green"; then
      if [ "$held" = 1 ]; then cell="held"; else cell="next merge"; fi
    else cell="waits for CI"; fi
    rows+="| $(row_label "$c") | $(ci_cell "$c") | $cell |"$'\n'
  done < <(git rev-list --first-parent origin/main)

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
  QUEUE+=$'. The next merge takes `main` up to its newest commit whose CI passed.\n\n'
  QUEUE+=$'| Pull request | CI | Production |\n| --- | --- | --- |\n'
  QUEUE+=$rows
}

# --- the countdown issue ---------------------------------------------------------------------

ensure_label() {
  gh label create "$RELEASE_LABEL" --color 5319e7 --force \
    --description "The merge of main into production (promote-production.yml)" >/dev/null
}
open_issues() {
  gh issue list --state open --label "$RELEASE_LABEL" --limit 100 --json number --jq 'map(.number) | sort | .[]'
}
# The countdown issue: the oldest open one, which stays open for good.
the_issue() { open_issues | head -n1; }

# The first RELEASE_HOUR in RELEASE_TZ at least MIN_GAP_HOURS after $1 (seconds since the epoch).
# Days are counted on the local calendar, and each candidate's own offset applies, so it stays
# 10:00 AM local across a daylight-saving change (which happens at 2 AM, before any candidate).
next_due() {
  local floor=$(( $1 + MIN_GAP_HOURS * 3600 )) here day naive t
  here=$(( $1 + $(offset "$1") ))
  day=$(( here - here % 86400 ))
  while :; do
    naive=$(( day + RELEASE_HOUR * 3600 ))
    t=$(( naive - $(offset "$naive") ))
    [ "$t" -lt "$floor" ] || break
    day=$(( day + 86400 ))
  done
  echo "$t"
}

# The issue's state lives in globals: due, held and last, and its title and body as GitHub has them.
# A fresh countdown:
fresh_state() { due=$(next_due "$(now)") held=0 last=0; }

title_for() {
  local when hours
  when=$(stamp "$due")
  if [ "$held" = 1 ]; then echo "Merging to production: on hold (was due $when)"; return; fi
  hours=$(( (due - $(now) + 1800) / 3600 ))
  if [ "$hours" -le 0 ]; then echo "Merging to production at the next check ($when)"
  elif [ "$hours" -eq 1 ]; then echo "Merging to production in 1 hour ($when)"
  else echo "Merging to production in $hours hours ($when)"; fi
}

body_for() {
  printf '<!-- promote-production due=%s held=%s last=%s -->\n' "$due" "$held" "$last"
  if [ "$held" = 1 ]; then
    printf '**Production: on hold.** `main` will not merge into `production` until someone comments `/resume`. It was due %s.\n' "$(stamp "$due")"
  else
    printf '**Production:** `main` merges into `production`, which is what players are served (the site and its server, together), at **%s**, through a pull request.\n' "$(stamp "$due")"
  fi
  printf '\n**Staging** deploys every commit of `main` whose CI passed, by itself (`deploy-staging.yml`): nothing here holds it.\n'
  printf '%s' "$QUEUE"
  cat <<'TEXT'

### Commands

Comment here to change it. Only people with write access are heard; a command gets a 👍, or a 😕 when it was refused.

| Comment | Effect |
| --- | --- |
| `/hold` and a reason, if you like | Nothing merges into production until `/resume`. |
| `/resume` | Lifts the hold. If the time has passed, it merges at the next check, which a comment starts at once. |
TEXT
  printf '| `/delay 3h`, `/delay 2d` | Moves the merge later by that long, at most %s hours at a time. Past the time already, it counts from the comment. |\n' "$MAX_DELAY_HOURS"
  printf '| `/fast-forward` | Merges now, past a hold. |\n'
  printf '\nThe title counts down: every check (hourly, and on every green CI run on `main`) brings its hours, and the queue, up to date.\n'
  cat <<'TEXT'

This issue stays open: when a merge goes out it says so below and counts down to the next. A change to the card catalog waits like any other: production's site and server go out together, so they always agree on it. For anything urgent, comment `/fast-forward`, or run *promote production* from the Actions tab: either merges the newest green commit at once.

Plan: `docs/deploy-cloudflare.md`. Written by `scripts/promote-production.sh`; the first line is its state, so leave it be.
TEXT
}

create_issue() {
  local f url
  f=$(mktemp)
  body_for > "$f"
  ensure_label
  url=$(try gh issue create --title "$(title_for)" --label "$RELEASE_LABEL" --label "$HUMAN_LABEL" \
    --assignee "$ASSIGNEES" --body-file "$f")
  rm -f "$f"
  echo "Opened ${url##*/}: the next merge is at $(stamp "$due")."
}

# Once the merge was due: says on issue $1 what became of it, and counts down to the next.
settle() {
  local msg
  case $RESULT in
    merged) msg="Merged to production in #$PR_NUMBER ($WHY): $COUNT commits up to \`${CANDIDATE:0:7}\`, catalog $CATALOG." ;;
    created) msg="Created \`production\` at \`${CANDIDATE:0:7}\`, catalog $CATALOG ($WHY)." ;;
    *) msg="Nothing to merge: \`production\` already has every green commit of \`main\`." ;;
  esac
  due=$(next_due "$(now)") held=0
  refresh_queue
  save_issue "$1"
  gh issue comment "$1" --body "$msg The next merge is at $(stamp "$due"). $(run_link)"
}

# A state line written while Vercel was staging also carries `vercel_held` and `vercel_until`; it
# reads the same, and the next save writes it without them.
load_issue() {
  local m info
  info=$(try gh issue view "$1" --json title,body)
  title=$(jq -r .title <<<"$info")
  body=$(jq -r .body <<<"$info" | tr -d '\r')
  m=$(grep -o 'promote-production due=[0-9]* held=[01] last=[0-9]*' <<<"$body" | head -n1 || true)
  if [ -z "$m" ]; then
    echo "#$1 has lost its state line; scheduling it afresh."
    fresh_state
    return
  fi
  due=${m#*due=}; due=${due%% *}
  held=${m#*held=}; held=${held%% *}
  last=${m#*last=}
}
save_issue() { # issue
  local f want text
  want=$(title_for)
  text=$(body_for)
  # The body is written when its text changed (new state, the queue, or a new wording such as the
  # time zone); the title when its "in N hours" did, which is every check that crossed an hour, so
  # it counts down.
  if [ "$text" != "$body" ]; then
    f=$(mktemp)
    printf '%s\n' "$text" > "$f"
    if [ "$want" != "$title" ]; then
      try gh issue edit "$1" --title "$want" --body-file "$f"
    else
      try gh issue edit "$1" --body-file "$f"
    fi
    rm -f "$f"
  elif [ "$want" != "$title" ]; then
    try gh issue edit "$1" --title "$want"
  fi
  title=$want body=$text
}
react() { gh api -X POST "repos/$REPO/issues/comments/$1/reactions" -f content="$2" --silent >/dev/null || true; }

# Reads the comments on issue $1 newer than `last` and applies each command to due and held.
apply_commands() {
  local line id cmdline cmd arg first rest n unit hours at base
  while IFS= read -r line; do
    id=$(jq -r .id <<<"$line")
    [ "$id" -gt "$last" ] || continue
    last=$id
    cmdline=$(jq -r .body <<<"$line" | tr -d '\r' \
      | grep -iE -m1 '^[[:space:]]*/(hold|resume|release|unhold|delay|fast-forward)([[:space:]]|$)' || true)
    [ -n "$cmdline" ] || continue
    read -r cmd arg <<<"$cmdline"
    cmd=$(tr '[:upper:]' '[:lower:]' <<<"${cmd#/}")
    if ! jq -e '(.assoc == "OWNER" or .assoc == "MEMBER" or .assoc == "COLLABORATOR") and (.bot | not)' <<<"$line" >/dev/null; then
      echo "Comment $id: /$cmd from $(jq -r .login <<<"$line") ignored: not someone with write access."
      react "$id" confused
      continue
    fi
    # A first word naming production changes nothing; one naming staging is refused (the header
    # says why). What follows is the reason or the length.
    read -r first rest <<<"$arg"
    case $(tr '[:upper:]' '[:lower:]' <<<"$first") in
      production | prod | cloudflare) arg=$rest ;;
      staging | vercel)
        echo "Comment $id: /$cmd $first refused: staging deploys every green commit of main by itself (deploy-staging.yml), and this issue holds production only."
        react "$id" confused
        continue
        ;;
    esac
    at=$(jq -r '.at | fromdateiso8601' <<<"$line")
    hours=0
    if [ "$cmd" = delay ]; then
      if [[ $arg =~ ^([0-9]{1,4})[[:space:]]*(h|hr|hrs|hour|hours|d|day|days)?([[:space:]].*)?$ ]]; then
        n=$((10#${BASH_REMATCH[1]})) unit=$(tr '[:upper:]' '[:lower:]' <<<"${BASH_REMATCH[2]}")
        hours=$n
        case $unit in d | day | days) hours=$((n * 24)) ;; esac
        if [ "$hours" -lt 1 ] || [ "$hours" -gt "$MAX_DELAY_HOURS" ]; then
          echo "Comment $id: /delay of $hours hours is outside 1 to $MAX_DELAY_HOURS; use /hold for longer."
          react "$id" confused
          continue
        fi
      else
        echo "Comment $id: /delay needs a number of hours or days, like '/delay 3h' or '/delay 2d'."
        react "$id" confused
        continue
      fi
    fi
    case $cmd in
      hold) held=1 ;;
      resume | release | unhold) held=0 ;;
      # Due the moment it was written, and no longer held: this same check merges.
      fast-forward) [ "$at" -ge "$due" ] || due=$at; held=0 ;;
      # Past the time already, it counts from the comment.
      delay) base=$due; [ "$at" -le "$base" ] || base=$at; due=$((base + hours * 3600)) ;;
    esac
    react "$id" '+1'
  done < <(gh api "repos/$REPO/issues/$1/comments" --paginate \
    --jq '.[] | {id, login: .user.login, bot: (.user.type == "Bot"), assoc: .author_association, at: .created_at, body} | tojson')
}

# --- the events ------------------------------------------------------------------------------

# The queue, which refresh_queue writes.
QUEUE=''

check() {
  local issues issue n cand
  issues=$(open_issues)
  if [ -z "$issues" ]; then
    fresh_state
    refresh_queue
    create_issue
    return 0
  fi
  issue=$(head -n1 <<<"$issues")
  for n in $issues; do
    [ "$n" = "$issue" ] || gh issue close "$n" --reason "not planned" --comment "Superseded by #$issue, the countdown issue that stays open."
  done

  load_issue "$issue"
  apply_commands "$issue"
  refresh_queue
  save_issue "$issue"

  if [ "$held" = 1 ]; then echo "#$issue is on hold; nothing merges."; return 0; fi
  if [ "$(now)" -lt "$due" ]; then echo "#$issue: the next merge is at $(stamp "$due")."; return 0; fi
  cand=$(vet "")
  promote "$cand" "the scheduled merge" "$issue"
  [ "$RESULT" != waiting ] || return 0
  # Production has every green commit, but main is ahead of it: those commits' CI has not passed yet.
  # That is not "nothing to merge". The countdown stays due, and the next green run on main merges.
  if [ "$RESULT" = uptodate ] && [ "$(git rev-list --count origin/production..origin/main)" -gt 0 ]; then
    echo "::notice::production has every green commit of main, but main has $(git rev-list --count origin/production..origin/main) newer commit(s) whose CI has not passed yet. #$issue stays due: the next green CI run on main merges them."
    return 0
  fi
  settle "$issue"
}

manual() {
  local cand issue
  issue=$(the_issue)
  if [ -n "$issue" ]; then load_issue "$issue"; else fresh_state; fi

  cand=$(vet "${INPUT_SHA:-}")
  promote "$cand" "a manual run" "$issue"
  [ "$RESULT" != waiting ] || fail "Nothing merged: main's newest commit changes .github/workflows and its CI has not passed yet, and GitHub refuses this workflow's token a push of workflow files that differ from main's. Run this again once CI on main is green."
  # A merge by hand starts the countdown over.
  if [ "$RESULT" != uptodate ]; then
    if [ -n "$issue" ]; then settle "$issue"; return 0; fi
    due=$(next_due "$(now)") held=0
  fi

  refresh_queue
  if [ -n "$issue" ]; then save_issue "$issue"; else create_issue; fi
}

case $EVENT in
  schedule | issue_comment | workflow_run) check ;;
  workflow_dispatch) manual ;;
  *) fail "promote-production.sh does not handle the $EVENT event." ;;
esac
