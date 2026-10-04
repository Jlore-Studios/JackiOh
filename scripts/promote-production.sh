#!/usr/bin/env bash
# Merges main into production (Cloudflare) through a pull request, and keeps the issue that counts
# down to the next merge. .github/workflows/promote-production.yml runs it; docs/deploy-cloudflare.md
# is the plan and the runbook. apps/web/src/net/promote-production.test.ts holds it in place, against
# a throwaway repository and a stand-in for `gh`.
#
# The countdown issue. One open issue labelled RELEASE_LABEL says "Merging to production in N hours"
# and carries, in a hidden comment, the time (`due`), whether it is on hold (`held`) and the last
# comment it has read (`last`). Anyone with write access comments on it:
#   /hold [why]            nothing merges until /resume
#   /resume                lifts the hold; a merge already overdue goes out at the next check
#   /delay <n>[h|d]        moves the merge n hours (or days) later, at most MAX_DELAY_HOURS a time
# Each command gets a +1 reaction, or a confused one when it was refused (not from a collaborator, or
# not understood). The next merge time is the first RELEASE_HOUR_UTC at least MIN_GAP_HOURS ahead.
#
# What runs it, and what each does (EVENT is github.event_name):
#   schedule, issue_comment, workflow_run
#                            a check: first the catalog fast path (below), then read the new commands,
#                            merge when due and not on hold, close the issue as completed with the pull
#                            request and open the next one. workflow_run is a green CI run on main; it
#                            is here because GitHub has not started this repository's scheduled runs
#                            (CLAUDE.md, the night bot), so the first green push after the issue's time
#                            is what merges, and the cron is a second chance, not the mechanism.
#   catalog fast path        part of every check. The commit CI just passed (workflow_run), or the
#                            newest green one, merges at once when it changes render.yaml's
#                            CATALOG_VERSION. Render deploys apps/server from main on every push and
#                            the bundle compiles the catalog version in, so a catalog left a day
#                            behind would refuse every deck save and queue in production (SPEC §9.4,
#                            R105). It ignores the hold and leaves the countdown's time alone.
#   workflow_dispatch        by hand: the newest green commit of main, or INPUT_SHA, at once, hold or no
#                            hold. When it merged something the countdown starts over.
#
# The merge. The candidate is a commit of main whose push-to-main CI run passed (the newest, unless
# one is named). It is pushed as promote/<date>-<sha>, a pull request into production is opened with
# the commits listed, and it is merged with a merge commit. Because production only ever receives
# merges of green main commits, its files always equal the candidate's; a production that carries
# anything else (someone committed to it) stops the run before anything is opened, and so does a
# result that differs from the candidate. Nothing is ever force-pushed. If production does not exist
# the candidate is pushed as production, with no pull request.
#
# It talks to GitHub through `gh` (GH_TOKEN) and to git through the checkout's own credentials: the
# workflow checks out main with its whole history. A push or a merge made with GITHUB_TOKEN starts no
# workflow (so this cannot loop), but GitHub still sends it to Cloudflare's app, which builds
# production. Needs GNU or BSD coreutils only for `date +%s`; the clock arithmetic is plain seconds
# and jq does the formatting.

# The backticks in the printf formats below are Markdown for GitHub, not command substitutions.
# shellcheck disable=SC2016
set -euo pipefail

: "${GITHUB_REPOSITORY:?}"
: "${EVENT:?}"
: "${RELEASE_LABEL:=production merge}"   # keep equal to the label named in promote-production.yml's `if`
: "${RELEASE_HOUR_UTC:=15}"
: "${MIN_GAP_HOURS:=12}"
: "${MAX_DELAY_HOURS:=168}"
: "${LISTED_COMMITS:=100}"
: "${MERGE_ATTEMPTS:=4}"
: "${MERGE_RETRY_SECONDS:=5}"
REPO=$GITHUB_REPOSITORY
SUMMARY=${GITHUB_STEP_SUMMARY:-/dev/null}

now() { echo "${NOW:-$(date -u +%s)}"; }
stamp() { jq -nr --argjson t "$1" '$t | strftime("%Y-%m-%d %H:%M UTC")'; }
fail() { echo "::error::$*" >&2; exit 1; }
run_link() { [ -n "${GITHUB_RUN_ID:-}" ] && echo "([run](${GITHUB_SERVER_URL:-https://github.com}/$REPO/actions/runs/$GITHUB_RUN_ID))" || true; }

# --- git -------------------------------------------------------------------------------------

catalog() {
  git show "$1:render.yaml" 2>/dev/null | awk '$1 == "-" && $2 == "key:" { k = $3; next }
    k == "CATALOG_VERSION" && $1 == "value:" { gsub(/"/, "", $2); print $2; exit }' || true
}
has_production() { git rev-parse --verify -q origin/production >/dev/null; }

newest_green() {
  gh run list --workflow ci.yml --branch main --event push --status success --limit 1 \
    --json headSha --jq '.[0].headSha // ""'
}
# Prints the full sha of a commit of main whose push-to-main CI run passed: $1, or the newest.
vet() {
  local c=$1
  [ -n "$c" ] || c=$(newest_green)
  [ -n "$c" ] || fail "No push to main has a passing CI run; nothing can be promoted."
  c=$(git rev-parse --verify "$c^{commit}") || fail "$1 is not a commit."
  git merge-base --is-ancestor "$c" origin/main || fail "$c is not on main."
  [ -n "$(gh run list --workflow ci.yml --branch main --event push --status success \
    --commit "$c" --limit 1 --json headSha --jq '.[].headSha')" ] \
    || fail "$c has no passing push-to-main CI run."
  echo "$c"
}

# Merges commit $1 of main into production. $2 says why (for the pull request and the issue's
# comment). Sets RESULT to merged, created or uptodate, and for a merge PR_NUMBER, COUNT, CANDIDATE,
# CATALOG and WHY.
promote() {
  local cand=$1 current base branch pr url body attempt
  WHY=$2 PR_NUMBER='' COUNT=0 CANDIDATE=$cand CATALOG=$(catalog "$cand")

  if ! has_production; then
    git push origin "$cand:refs/heads/production"
    RESULT=created
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
  base=$(git merge-base "$current" "$cand") || fail "production and $cand share no history."
  git diff --quiet "$base" "$current" || fail "production has diverged: it carries changes that are not on main (git diff ${base:0:12} ${current:0:12}). Not merging; see docs/deploy-cloudflare.md, 'production has diverged'."

  COUNT=$(git rev-list --count "$current..$cand")
  branch="promote/$(date -u +%Y%m%d)-${cand:0:7}"
  body=$(mktemp)
  {
    echo "Merges \`main\` into \`production\`, which Cloudflare builds and deploys."
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
  git push origin "$cand:refs/heads/$branch"
  pr=$(gh pr list --base production --head "$branch" --state open --json number --jq '.[0].number // empty')
  if [ -z "$pr" ]; then
    url=$(gh pr create --base production --head "$branch" --label "$RELEASE_LABEL" \
      --title "Promote main to production: $COUNT commit(s) up to ${cand:0:7}" --body-file "$body")
    pr=${url##*/}
  fi
  rm -f "$body"

  # GitHub works out whether a new pull request merges cleanly in the background, so the first
  # attempt right after `create` can be refused for a moment.
  for attempt in $(seq 1 "$MERGE_ATTEMPTS"); do
    if gh pr merge "$pr" --merge --match-head-commit "$cand"; then break; fi
    [ "$attempt" -lt "$MERGE_ATTEMPTS" ] || fail "Pull request #$pr could not be merged into production. It stays open; read its page for the reason."
    sleep "$MERGE_RETRY_SECONDS"
  done
  gh api -X DELETE "repos/$REPO/git/refs/heads/$branch" --silent || true

  git fetch -q origin production
  git diff --quiet "$cand" origin/production \
    || fail "production's files differ from ${cand:0:12} after merging #$pr. Someone changed production meanwhile; see docs/deploy-cloudflare.md, 'production has diverged'."

  RESULT=merged PR_NUMBER=$pr
  echo "Merged #$pr: production now has ${cand:0:12} ($COUNT commits)."
  printf '### production <- `%s` (%s)\n\nPull request #%s, catalog %s, %s commits.\n' \
    "${cand:0:12}" "$WHY" "$pr" "$CATALOG" "$COUNT" >> "$SUMMARY"
}

# A catalog bump is promoted ahead of the schedule and past any hold. $1 is the commit CI passed, or
# empty for the newest green one. Does nothing when production does not exist yet (the first merge
# creates it) or when the commit does not change the catalog.
catalog_gate() {
  local cand=$1 current issue
  [ -n "$cand" ] || cand=$(newest_green)
  [ -n "$cand" ] || return 0
  cand=$(vet "$cand")
  has_production || return 0
  current=$(git rev-parse origin/production)
  git merge-base --is-ancestor "$cand" "$current" && return 0
  if [ "$(catalog "$current")" = "$(catalog "$cand")" ]; then
    echo "No catalog change ($(catalog "$cand")); this waits for the scheduled merge."
    return 0
  fi
  promote "$cand" "a catalog bump to $(catalog "$cand"), which is never held"
  issue=$(open_issues | tail -n1)
  if [ -n "$issue" ] && [ "$RESULT" = merged ]; then
    gh issue comment "$issue" --body "Catalog $CATALOG could not wait: it was merged to production ahead of schedule in #$PR_NUMBER. The scheduled merge above still stands for everything else. $(run_link)"
  fi
}

# --- the countdown issue ---------------------------------------------------------------------

ensure_label() {
  gh label create "$RELEASE_LABEL" --color 5319e7 --force \
    --description "The merge of main into production (promote-production.yml)" >/dev/null
}
open_issues() {
  gh issue list --state open --label "$RELEASE_LABEL" --limit 100 --json number --jq 'map(.number) | sort | .[]'
}

# The first RELEASE_HOUR_UTC at least MIN_GAP_HOURS after $1 (seconds since the epoch).
next_due() {
  local floor=$(( $1 + MIN_GAP_HOURS * 3600 )) t=$(( $1 - $1 % 86400 + RELEASE_HOUR_UTC * 3600 ))
  while [ "$t" -lt "$floor" ]; do t=$(( t + 86400 )); done
  echo "$t"
}

title_for() { # due held
  local when hours
  when=$(stamp "$1")
  if [ "$2" = 1 ]; then echo "Merging to production: on hold (was due $when)"; return; fi
  hours=$(( ($1 - $(now) + 1800) / 3600 ))
  if [ "$hours" -le 0 ]; then echo "Merging to production at the next check ($when)"
  elif [ "$hours" -eq 1 ]; then echo "Merging to production in 1 hour ($when)"
  else echo "Merging to production in $hours hours ($when)"; fi
}

body_for() { # due held last
  printf '<!-- promote-production due=%s held=%s last=%s -->\n' "$1" "$2" "$3"
  if [ "$2" = 1 ]; then
    printf '**On hold.** `main` will not merge into `production` until someone comments `/resume`. It was due %s.\n' "$(stamp "$1")"
  else
    printf '`main` merges into `production`, which is what Cloudflare serves, at **%s**, through a pull request.\n' "$(stamp "$1")"
  fi
  cat <<'EOF'

Comment here to change that. Only people with write access are heard; a command gets a 👍, or a 😕 when it was refused.

| Comment | Effect |
| --- | --- |
| `/hold` and a reason, if you like | Nothing merges until `/resume`. |
| `/resume` | Lifts the hold. If the time has passed, it merges at the next check, which a comment starts at once. |
EOF
  printf '| `/delay 3h`, `/delay 2d` | Moves the merge later by that long, at most %s hours at a time. Past the time already, it counts from the comment. |\n' "$MAX_DELAY_HOURS"
  cat <<'EOF'

When it merges, this issue is closed as completed with a link to the pull request, and the next one is opened. A change to the card catalog (`CATALOG_VERSION` in `render.yaml`) is the exception: the server already runs it, so it merges as soon as CI passes and a hold does not stop it. For anything else urgent, run *promote production* from the Actions tab: it merges the newest green commit at once.

Plan: `docs/deploy-cloudflare.md`. Written by `scripts/promote-production.sh`; the first line is its state, so leave it be.
EOF
}

create_issue() { # due
  local f url
  f=$(mktemp)
  body_for "$1" 0 0 > "$f"
  ensure_label
  url=$(gh issue create --title "$(title_for "$1" 0)" --label "$RELEASE_LABEL" --body-file "$f")
  rm -f "$f"
  echo "Opened ${url##*/}: the next merge is at $(stamp "$1")."
}

# Closes issue $1 as completed, saying what became of the merge, and opens the next one.
roll() {
  local msg
  case $RESULT in
    merged) msg="Merged to production in #$PR_NUMBER ($WHY): $COUNT commits up to \`${CANDIDATE:0:7}\`, catalog $CATALOG. $(run_link)" ;;
    created) msg="Created \`production\` at \`${CANDIDATE:0:7}\`, catalog $CATALOG ($WHY). $(run_link)" ;;
    *) msg="Nothing to merge: \`production\` already has every green commit of \`main\`. $(run_link)" ;;
  esac
  gh issue close "$1" --reason completed --comment "$msg"
  create_issue "$(next_due "$(now)")"
}

# The issue's state lives in globals: due, held, last (and changed, retitle for what to write back).
load_issue() {
  local m
  m=$(gh issue view "$1" --json body --jq .body | grep -o 'promote-production due=[0-9]* held=[01] last=[0-9]*' | head -n1 || true)
  if [ -z "$m" ]; then
    echo "#$1 has lost its state line; scheduling it afresh."
    due=$(next_due "$(now)") held=0 last=0 changed=1 retitle=1
    return
  fi
  due=${m#*due=}; due=${due%% *}
  held=${m#*held=}; held=${held%% *}
  last=${m#*last=}
}
save_issue() { # issue
  local f
  f=$(mktemp)
  body_for "$due" "$held" "$last" > "$f"
  if [ "$retitle" = 1 ]; then
    gh issue edit "$1" --title "$(title_for "$due" "$held")" --body-file "$f"
  else
    gh issue edit "$1" --body-file "$f"
  fi
  rm -f "$f"
}
react() { gh api -X POST "repos/$REPO/issues/comments/$1/reactions" -f content="$2" --silent >/dev/null || true; }

# Reads the comments on issue $1 newer than `last` and applies each command to due and held.
apply_commands() {
  local line id cmdline cmd arg n unit hours at base
  while IFS= read -r line; do
    id=$(jq -r .id <<<"$line")
    [ "$id" -gt "$last" ] || continue
    last=$id changed=1
    cmdline=$(jq -r .body <<<"$line" | tr -d '\r' \
      | grep -iE -m1 '^[[:space:]]*/(hold|resume|release|unhold|delay)([[:space:]]|$)' || true)
    [ -n "$cmdline" ] || continue
    read -r cmd arg <<<"$cmdline"
    cmd=$(tr '[:upper:]' '[:lower:]' <<<"${cmd#/}")
    if ! jq -e '(.assoc == "OWNER" or .assoc == "MEMBER" or .assoc == "COLLABORATOR") and (.bot | not)' <<<"$line" >/dev/null; then
      echo "Comment $id: /$cmd from $(jq -r .login <<<"$line") ignored: not someone with write access."
      react "$id" confused
      continue
    fi
    case $cmd in
      hold) [ "$held" = 1 ] || { held=1 retitle=1; } ;;
      resume | release | unhold) [ "$held" = 0 ] || { held=0 retitle=1; } ;;
      delay)
        if [[ $arg =~ ^([0-9]{1,4})[[:space:]]*(h|hr|hrs|hour|hours|d|day|days)?([[:space:]].*)?$ ]]; then
          n=$((10#${BASH_REMATCH[1]})) unit=$(tr '[:upper:]' '[:lower:]' <<<"${BASH_REMATCH[2]}")
          hours=$n
          case $unit in d | day | days) hours=$((n * 24)) ;; esac
          if [ "$hours" -lt 1 ] || [ "$hours" -gt "$MAX_DELAY_HOURS" ]; then
            echo "Comment $id: /delay of $hours hours is outside 1 to $MAX_DELAY_HOURS; use /hold for longer."
            react "$id" confused
            continue
          fi
          at=$(jq -r '.at | fromdateiso8601' <<<"$line")
          base=$due
          [ "$at" -le "$base" ] || base=$at
          due=$((base + hours * 3600)) retitle=1
        else
          echo "Comment $id: /delay needs a number of hours or days, like '/delay 3h'."
          react "$id" confused
          continue
        fi
        ;;
    esac
    react "$id" '+1'
  done < <(gh api "repos/$REPO/issues/$1/comments" --paginate \
    --jq '.[] | {id, login: .user.login, bot: (.user.type == "Bot"), assoc: .author_association, at: .created_at, body} | tojson')
}

# --- the events ------------------------------------------------------------------------------

check() {
  local issues issue n cand
  catalog_gate "${RUN_SHA:-}"
  issues=$(open_issues)
  if [ -z "$issues" ]; then
    create_issue "$(next_due "$(now)")"
    return 0
  fi
  issue=$(tail -n1 <<<"$issues")
  for n in $issues; do
    [ "$n" = "$issue" ] || gh issue close "$n" --reason "not planned" --comment "Superseded by #$issue."
  done

  changed=0 retitle=0
  load_issue "$issue"
  apply_commands "$issue"
  [ "$changed" = 0 ] || save_issue "$issue"

  if [ "$held" = 1 ]; then echo "#$issue is on hold; nothing merges."; return 0; fi
  if [ "$(now)" -lt "$due" ]; then echo "#$issue: the next merge is at $(stamp "$due")."; return 0; fi
  cand=$(vet "")
  promote "$cand" "the scheduled merge" "$issue"
  roll "$issue"
}

manual() {
  local cand issue
  cand=$(vet "${INPUT_SHA:-}")
  issue=$(open_issues | tail -n1)
  promote "$cand" "a manual run" "$issue"
  [ "$RESULT" != uptodate ] || return 0
  if [ -n "$issue" ]; then roll "$issue"; else create_issue "$(next_due "$(now)")"; fi
}

case $EVENT in
  schedule | issue_comment | workflow_run) check ;;
  workflow_dispatch) manual ;;
  *) fail "promote-production.sh does not handle the $EVENT event." ;;
esac
