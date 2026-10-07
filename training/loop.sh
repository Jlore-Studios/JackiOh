#!/bin/sh
# training/loop.sh <lane>: one AI training lane, forever (docs/v0.3.0/README.md §8; training/README.md).
#
# The training box runs it as the systemd service jackioh-train@<lane> (Restart=always), as the
# lane's own Linux user (agent-train-<lane>) with its own checkout, Devin login and GitHub token.
# Each cycle:
#
#   1. fetch main; reset the lane's branch ai/<lane> to it; build jackioh from it and keep that
#      binary as the parent (~/parent-jackioh): the parent is always the AI on main;
#   2. run one Devin session on the lane's standing prompt (training/<lane>.md);
#   3. when the session ends with a promotion commit, rebase it on main. If main's AI (or the engine
#      and cards it plays on) changed meanwhile, the parent changed: the promotion is dropped and the
#      cycle starts again at 1. Otherwise re-check it against the parent (`promote --verify`), push
#      ai/<lane> and open a pull request titled with the promotion commit's subject
#      (`AI gen <N> (<lane>): …`) and the gate's report as its body, with auto-merge on;
#   4. sleep and repeat.
#
# Each cycle also loads the finished days' game records (~/training-out/<lane>/<date>.jsonl) into
# Postgres with `jackioh-server stats-import`, once per file, when the box has DATABASE_URL. While a
# pull request from ai/<lane> is still open, the cycle waits for it. Everything goes to
# ~/logs/<lane>.log.
#
# Stop a lane with `sudo systemctl stop jackioh-train@<lane>` (and `disable` to keep it stopped).
#
# Environment: DEVIN_MODEL (required), GH_TOKEN (the lane user's own, for gh), DATABASE_URL
# (optional), TRAIN_PAUSE_SECONDS (default 60), TRAIN_SESSION_SECONDS (default 16200: the standing
# prompt's four hours, plus half an hour for the last gate run to finish).

set -u

lane="${1:-}"
case "$lane" in
  improve | unban) ;;
  *)
    echo "usage: training/loop.sh improve|unban" >&2
    exit 2
    ;;
esac

# Step 1 resets the checkout this file lives in, so the loop runs from a copy of itself; a cycle
# that finds a new loop.sh on main exits, and systemd starts the new one.
if [ -z "${JACKIOH_LOOP_COPY:-}" ]; then
  repo=$(cd "$(dirname "$0")/.." && pwd) || exit 1
  copy="$HOME/.cache/jackioh-train/loop-$lane.sh"
  mkdir -p "$(dirname "$copy")" || exit 1
  cp "$0" "$copy" || exit 1
  JACKIOH_LOOP_COPY="$copy"
  JACKIOH_REPO="$repo"
  export JACKIOH_LOOP_COPY JACKIOH_REPO
  exec sh "$copy" "$lane"
fi
repo="${JACKIOH_REPO:?}"
copy="$JACKIOH_LOOP_COPY"

branch="ai/$lane"
logs="$HOME/logs"
out="$HOME/training-out/$lane"
parent="$HOME/parent-jackioh"
pause="${TRAIN_PAUSE_SECONDS:-60}"
session_limit="${TRAIN_SESSION_SECONDS:-16200}"

mkdir -p "$logs" "$out" || exit 1
exec >>"$logs/$lane.log" 2>&1

# `cargo jackioh arena` and `promote` append every game they play to this directory (R376).
JACKIOH_TRAINING_OUT="$out"
export JACKIOH_TRAINING_OUT

log() {
  printf '%s %s: %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$lane" "$*"
}

if [ -z "${DEVIN_MODEL:-}" ]; then
  log "DEVIN_MODEL is not set; the lane cannot run"
  exit 2
fi

# Loads every finished day's record file not loaded yet (today's is still being written).
import_records() {
  [ -n "${DATABASE_URL:-}" ] || return 0
  today=$(date -u +%Y-%m-%d)
  loaded="$out/imported.txt"
  for file in "$out"/[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9].jsonl; do
    [ -f "$file" ] || continue
    name=$(basename "$file")
    [ "$name" = "$today.jsonl" ] && continue
    if [ -f "$loaded" ] && grep -qxF "$name" "$loaded"; then
      continue
    fi
    if cargo run --release --quiet -p jackioh-server -- stats-import "$file"; then
      echo "$name" >>"$loaded"
      log "loaded $name into Postgres"
    else
      log "stats-import failed on $name; it is tried again next cycle"
    fi
  done
}

# Whether the session left a promotion on ai/<lane>: a commit after $base whose subject is
# `AI gen …`, which changes crates/ai/generation.json, and which changes nothing outside
# crates/ai/ and training/history/ (CI refuses anything else from an ai/* branch).
promoted() {
  [ "$(git rev-parse HEAD)" != "$base" ] || return 1
  case "$(git log -1 --format=%s)" in
    "AI gen "*) ;;
    *) return 1 ;;
  esac
  if git diff --quiet "$base" HEAD -- crates/ai/generation.json; then
    return 1
  fi
  outside=$(git diff --name-only "$base" HEAD | grep -v -e '^crates/ai/' -e '^training/history/')
  if [ -n "$outside" ]; then
    log "the promotion commit changes files outside crates/ai/ and training/history/: $outside"
    return 1
  fi
  return 0
}

cycle() {
  cd "$repo" || return 1

  open_pr=$(gh pr list --head "$branch" --state open --json number --jq '.[0].number // empty') || {
    log "gh pr list failed; waiting"
    return 1
  }
  if [ -n "$open_pr" ]; then
    log "pull request #$open_pr from $branch is still open; waiting for it"
    return 0
  fi

  # 1. main, the lane's branch on it, and the parent built from it.
  git rebase --abort >/dev/null 2>&1
  git fetch --quiet origin main || {
    log "git fetch failed"
    return 1
  }
  git reset --quiet --hard || return 1
  git clean -fdq || return 1
  git checkout --quiet -B "$branch" origin/main || {
    log "cannot reset $branch to origin/main"
    return 1
  }
  if ! cmp -s training/loop.sh "$copy"; then
    log "training/loop.sh changed on main; restarting on the new one"
    exit 0
  fi
  base=$(git rev-parse HEAD)
  if ! cargo build --release --quiet -p jackioh-tools; then
    log "jackioh does not build on main at $base"
    return 1
  fi
  cp target/release/jackioh "$parent.new" && mv "$parent.new" "$parent" || {
    log "cannot keep the parent binary at $parent"
    return 1
  }
  log "parent built from main at $base"

  import_records

  # 2. One Devin session on the standing prompt.
  stamp=$(date -u +%Y%m%dT%H%M%SZ)
  log "Devin session $stamp starting on training/$lane.md"
  timeout "$session_limit" devin -p --prompt-file "training/$lane.md" --model "$DEVIN_MODEL" \
    --permission-mode dangerous --respect-workspace-trust false --export "$logs/$lane-$stamp.json"
  log "Devin session $stamp ended with status $?"

  # 3. A promotion: rebase, re-check against the parent, push and open the pull request.
  if ! promoted; then
    log "no promotion this session"
    return 0
  fi
  title=$(git log -1 --format=%s)
  git fetch --quiet origin main || {
    log "git fetch failed before the rebase"
    return 1
  }
  if ! git diff --quiet "$base" origin/main -- crates Cargo.toml Cargo.lock rust-toolchain.toml; then
    git branch --quiet -f "$branch-stale-$stamp" HEAD
    printf '\n- %s (loop): promotion %s, "%s", dropped: main moved under it, so its parent changed. Kept as the local branch %s.\n' \
      "$stamp" "$(git rev-parse --short HEAD)" "$title" "$branch-stale-$stamp" >>"$out/attempts.md"
    log "main's AI or engine moved since $base: \"$title\" is dropped (kept as $branch-stale-$stamp); starting again"
    return 0
  fi
  if ! git rebase --quiet origin/main; then
    git rebase --abort >/dev/null 2>&1
    log "\"$title\" does not rebase on main"
    return 1
  fi
  body="$out/promote-$stamp.md"
  if ! cargo jackioh promote --lane "$lane" --parent-bin "$parent" --verify >"$body"; then
    log "\"$title\" does not re-check against the parent (see $body); no pull request"
    return 0
  fi
  git fetch --quiet origin "+refs/heads/$branch:refs/remotes/origin/$branch" >/dev/null 2>&1
  git push --quiet --force-with-lease origin "$branch" || {
    log "cannot push $branch"
    return 1
  }
  url=$(gh pr create --base main --head "$branch" --title "$title" --body-file "$body") || {
    log "gh pr create failed for \"$title\""
    return 1
  }
  log "opened $url: $title"
  gh pr merge "$url" --auto --squash || log "auto-merge could not be turned on for $url"
  return 0
}

log "lane started in $repo"
while :; do
  cycle || log "the cycle stopped early; trying again after the pause"
  sleep "$pause"
done
