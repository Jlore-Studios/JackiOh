#!/bin/sh
# training/loop.sh <lane>: one AI training lane, forever (docs/v0.3.0/README.md §8; training/README.md).
#
# The training box runs it as the systemd service jackioh-train@<lane> (Restart=always), as the
# lane's own Linux user (agent-train-<lane>) with its own checkout, Devin login and GitHub token.
# The lane keeps one pull request open from ai/<lane>: a draft while Devin works, which shows the
# work as it goes, and ready for review, with auto-merge on, once a promotion is in it. Each cycle:
#
#   1. a promotion waiting on CI (the pull request is ready): if main moved under it, land it on
#      main again and push; if a required check failed, set it back (to draft, with the failure
#      noted in attempts.md for the next session); otherwise wait. Nothing else runs meanwhile;
#   2. otherwise fetch main, reset the lane's branch ai/<lane> to it, build jackioh from it and keep
#      that binary as the parent (~/parent-jackioh): the parent is always the AI on main. Push
#      ai/<lane> as main plus an empty `[skip ci]` commit naming the session, and make the lane's
#      pull request this session's draft (opening one when none is open);
#   3. run one Devin session on the lane's standing prompt (training/<lane>.md). Every
#      TRAIN_PROGRESS_SECONDS, push the checkout as it is to ai/<lane> as one `[skip ci]` commit
#      when it changed, and rewrite the session's comment on the draft: how long it has run, the
#      diff against main and what attempts.md gained;
#   4. when the session ends with a promotion commit, land it on main (`land`), run the web's tests
#      that play the AI when the box has pnpm, push ai/<lane>, retitle the pull request with the
#      promotion's subject (`AI gen <N> (<lane>): …`), give it the gate's report as its body, mark
#      it ready and turn on auto-merge. A promotion that fails any of that is set back;
#   5. sleep and repeat.
#
# Each cycle also loads the finished days' game records (~/training-out/<lane>/<date>.jsonl) into
# Postgres with `jackioh-server stats-import`, once per file, when the box has DATABASE_URL.
# Everything goes to ~/logs/<lane>.log.
#
# Stop a lane with `sudo systemctl stop jackioh-train@<lane>` (and `disable` to keep it stopped).
#
# Environment: DEVIN_MODEL (required), GH_TOKEN (the lane user's own, for gh), DATABASE_URL
# (optional), TRAIN_PAUSE_SECONDS (default 60), TRAIN_SESSION_SECONDS (default 16200: the standing
# prompt's four hours, plus half an hour for the last gate run to finish), TRAIN_PROGRESS_SECONDS
# (default 900).

# shellcheck disable=SC2016 # the backticks in printf's formats are Markdown's, for GitHub
set -u

lane="${1:-}"
case "$lane" in
  improve | unban) ;;
  *)
    echo "usage: training/loop.sh improve|unban" >&2
    exit 2
    ;;
esac

# Step 2 resets the checkout this file lives in, so the loop runs from a copy of itself; a cycle
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
work="$HOME/.cache/jackioh-train/$lane"
parent="$HOME/parent-jackioh"
pause="${TRAIN_PAUSE_SECONDS:-60}"
session_limit="${TRAIN_SESSION_SECONDS:-16200}"
progress_every="${TRAIN_PROGRESS_SECONDS:-900}"

mkdir -p "$logs" "$out" "$work" || exit 1
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

slug=""
pr=""
stamp=""
why=""
report=""

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

# Whatever a run left in the checkout goes: a rebase, a cherry-pick, changes and untracked files.
clean_checkout() {
  git rebase --abort >/dev/null 2>&1
  git cherry-pick --abort >/dev/null 2>&1
  git reset --quiet --hard && git clean -fdq
}

# jackioh built from the checkout as it is, kept as the parent. Callers build it before they change
# anything the candidate compiles from, so cargo sees those changes as newer than this build.
build_parent() {
  cargo build --release --quiet -p jackioh-tools || return 1
  cp target/release/jackioh "$parent.new" && mv "$parent.new" "$parent"
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

# Lands the promotion commit $1 (one commit on top of the main its gate ran on, as the standing
# prompt and this loop make them) on origin/main as ai/<lane>, its gate proved there. When main's
# crates and toolchain did not move, the parent and the candidate are the very builds its gate
# measured, so the commit is only rebased (and, with $2 = verify, re-checked against the parent with
# `promote --verify`). Otherwise the parent changed: it is built from main again, the change (less
# the record the gate writes) is applied to main's AI, and the gate is played again for real, which
# renumbers the generation. Leaves the report, if any, in $report; returns 1 with the reason in
# $why (a clause, for set_back) when it cannot. Expects origin/main fetched.
land() {
  promo=$1
  report=""
  old_base=$(git rev-parse --verify --quiet "$promo^") || {
    why="it is not a commit on top of main"
    return 1
  }
  subject=$(git log -1 --format=%s "$promo")
  clean_checkout
  if git diff --quiet "$old_base" origin/main -- crates Cargo.toml Cargo.lock rust-toolchain.toml; then
    if ! { git checkout --quiet -B "$branch" "$promo" && git rebase --quiet origin/main; }; then
      git rebase --abort >/dev/null 2>&1
      why="it does not rebase on main"
      return 1
    fi
    [ "${2:-}" = verify ] || return 0
    report="$out/promote-$stamp.md"
    if ! cargo jackioh promote --lane "$lane" --parent-bin "$parent" --verify >"$report"; then
      why="it does not re-check against the parent (\`promote --verify\`; the report is $report on the box)"
      return 1
    fi
    return 0
  fi
  log "main's crates moved since $old_base: \"$subject\" is gated again on main"
  git checkout --quiet -B "$branch" origin/main || {
    why="the loop cannot check out main"
    return 1
  }
  if ! build_parent; then
    why="main does not build"
    return 1
  fi
  if ! git diff --binary "$old_base" "$promo" -- crates/ai ':(exclude)crates/ai/generation.json' | git apply --3way --whitespace=nowarn; then
    clean_checkout
    why="main's AI moved under it, and its change does not apply to main's AI"
    return 1
  fi
  on_main=$(jq -er .generation crates/ai/generation.json) || {
    why="crates/ai/generation.json on main has no generation"
    return 1
  }
  subject="AI gen $((on_main + 1)) ($lane): ${subject#*): }"
  if ! { printf '%s\n\n' "$subject"; git log -1 --format=%b "$promo"; } | git commit --quiet -F -; then
    why="the loop cannot commit it on main"
    return 1
  fi
  report="$out/promote-$stamp.md"
  if ! cargo jackioh promote --lane "$lane" --parent-bin "$parent" >"$report"; then
    why="main's AI, engine or cards moved under it, and against the new parent it fails its gate (the report is $report on the box)"
    return 1
  fi
  git add crates/ai/generation.json "training/history/$lane.jsonl" && git commit --quiet --amend --no-edit
}

# The web's tests that play the shipping AI on fixed seeds (the tutorial's lessons, R293, and
# practice), on the checkout, when the box has pnpm. CI runs them on every promotion too; they are
# the checks beyond its gate that a promotion is likeliest to fail. Without pnpm they are left to CI.
# Sets $web_failures when they fail.
web_check() {
  web_failures=""
  if ! command -v pnpm >/dev/null 2>&1; then
    log "no pnpm on the box: the web's tests are left to CI"
    return 0
  fi
  web_log="$logs/$lane-web-$stamp.log"
  if CYPRESS_INSTALL_BINARY=0 pnpm install --frozen-lockfile >"$web_log" 2>&1 \
    && sh scripts/build-wasm.sh >>"$web_log" 2>&1 \
    && pnpm --dir apps/web exec vitest run src/tutorial src/practice src/routes/practice >>"$web_log" 2>&1; then
    return 0
  fi
  web_failures=$(sed 's/\x1b\[[0-9;]*m//g' "$web_log" | grep -E '^ *FAIL ' | sort -u | head -n 20)
  [ -n "$web_failures" ] || web_failures="(none named: the install or the WASM build failed; $web_log on the box)"
  return 1
}

# Its input's lines joined with "; ".
joined() {
  awk 'NR > 1 { printf "; " } { printf "%s", $0 }'
}

# Posts the file $2 as a new comment on pull request $1 and prints the comment's id.
comment() {
  [ -n "$1" ] || return 1
  url=$(gh pr comment "$1" --body-file "$2") || {
    log "cannot comment on #$1"
    return 1
  }
  printf '%s\n' "${url##*issuecomment-}"
}

# Rewrites the comment with id $1 to the file $2.
rewrite() {
  [ -n "$1" ] || return 0
  gh api -X PATCH "repos/$slug/issues/comments/$1" -F "body=@$2" >/dev/null || log "cannot rewrite comment $1"
}

# The promotion at $1 does not go up, or comes back down: $2 says why, as a clause. Keeps the
# commit as a local branch for the next session, notes it in attempts.md (which the standing prompt
# has Devin read first), and turns pull request $3 back into a draft with a comment saying so.
set_back() {
  keep="$branch-set-back-$stamp"
  short=$(git rev-parse --short "$1")
  subject=$(git log -1 --format=%s "$1")
  git branch --quiet -f "$keep" "$1"
  printf '\n- %s (loop): promotion %s, "%s", was set back: %s. Its commit is the local branch `%s`.\n' \
    "$(date -u +%Y-%m-%dT%H:%MZ)" "$short" "$subject" "$2" "$keep" >>"$out/attempts.md"
  log "\"$subject\" ($short) set back: $2 (kept as $keep)"
  [ -n "$3" ] || return 0
  {
    printf '**Set back:** `%s` (%s): %s.\n\n' "$subject" "$short" "$2"
    printf 'The pull request is a draft again. The commit is kept on the box as the branch `%s`, and `attempts.md` says what happened, for the next session, which starts from `main`.\n' "$keep"
  } >"$work/note.md"
  comment "$3" "$work/note.md" >/dev/null
  gh pr merge "$3" --disable-auto >/dev/null 2>&1
  gh pr ready "$3" --undo >/dev/null 2>&1
}

# Puts the landed promotion (ai/<lane>, its report in $report) up on pull request $1: the web's
# tests, then the title, the body, the push, ready for review and auto-merge. $2 opens its comment.
publish() {
  head=$(git rev-parse HEAD)
  if ! web_check; then
    set_back "$head" "the web's tests that play the AI fail with it: $(printf '%s\n' "$web_failures" | sed 's/^ *FAIL *//' | joined)" "$1"
    return 0
  fi
  title=$(git log -1 --format=%s)
  if [ -z "$1" ]; then
    git push --quiet --force origin "$branch" || log "cannot push $branch"
    url=$(gh pr create --base main --head "$branch" --title "$title" --body-file "$report") || {
      log "gh pr create failed for \"$title\""
      return 1
    }
    set -- "${url##*/}" "$2"
  else
    if [ -n "$report" ]; then
      gh pr edit "$1" --title "$title" --body-file "$report" >/dev/null || log "cannot retitle #$1"
    else
      gh pr edit "$1" --title "$title" >/dev/null || log "cannot retitle #$1"
    fi
    git push --quiet --force origin "$branch" || {
      log "cannot push $branch"
      return 1
    }
  fi
  # A draft holding a promotion would read as the next session's draft, so a refusal sets it back.
  if ! gh pr ready "$1" >/dev/null 2>&1 && [ "$(gh pr view "$1" --json isDraft --jq .isDraft)" != false ]; then
    set_back "$(git rev-parse HEAD)" "GitHub would not mark its pull request ready for review (\`gh pr ready $1\`)" "$1"
    return 1
  fi
  gh pr merge "$1" --auto --squash || log "auto-merge could not be turned on for #$1; the next cycle tries again"
  {
    printf '**%s:** `%s` (%s) is up. CI runs every check now, the training gate included, and auto-merge lands it once they pass. If one fails, it comes back to draft and the next session starts from `main`.\n' \
      "$2" "$title" "$(git rev-parse --short HEAD)"
    if [ -n "$report" ]; then
      printf '\nThe gate on the box:\n\n'
      sed -n '/^|/p' "$report"
    fi
  } >"$work/note.md"
  comment "$1" "$work/note.md" >/dev/null
  log "#$1 is ready: $title"
}

# A promotion waiting on CI: pull request $1, whose head is $2, merge state $3, auto-merge armed $4.
watch() {
  git fetch --quiet origin main "+refs/heads/$branch:refs/remotes/origin/$branch" || {
    log "git fetch failed"
    return 1
  }
  if ! git show origin/main:training/loop.sh | cmp -s - "$copy"; then
    # systemd starts the file in the checkout, so the checkout goes to main first.
    clean_checkout
    git checkout --quiet -B "$branch" origin/main
    log "training/loop.sh changed on main; restarting on the new one"
    exit 0
  fi
  case "$(git log -1 --format=%s "$2" 2>/dev/null)" in
    "AI gen "*) ;;
    *)
      log "#$1 is ready for review without a promotion in it; it goes back to draft"
      gh pr ready "$1" --undo >/dev/null 2>&1
      return 0
      ;;
  esac
  failed=$(gh pr checks "$1" --required --json name,bucket,link \
    --jq '.[] | select(.bucket == "fail") | "\(.name) (\(.link))"' 2>/dev/null)
  # Main moved under it: GitHub wants it up to date (or it conflicts), or a check failed on a main
  # that has moved on since, whose fix may be on main now. Land it on main again; CI runs again.
  if [ "$3" = BEHIND ] || [ "$3" = DIRTY ] || { [ -n "$failed" ] && ! git merge-base --is-ancestor origin/main "$2"; }; then
    if land "$2"; then
      publish "$1" "Main moved, so the loop landed the promotion on it again"
    else
      set_back "$2" "$why" "$1"
    fi
    return 0
  fi
  if [ -n "$failed" ]; then
    set_back "$2" "CI failed it on $(printf '%s\n' "$failed" | joined). Read those checks' logs (\`gh pr checks $1\`, \`gh run view <run> --log-failed\`) before trying the change again" "$1"
    return 0
  fi
  if [ "$4" = false ]; then
    gh pr merge "$1" --auto --squash || log "auto-merge could not be turned on for #$1"
  fi
  log "#$1 ($(git rev-parse --short "$2")) waits on CI and auto-merge"
}

# Pushes the checkout as it is (HEAD and every change on it, committed or not) to ai/<lane> as one
# `[skip ci]` commit, when it differs from what is there, without touching Devin's HEAD or index.
push_wip() {
  index="$work/wip.index"
  rm -f "$index"
  wip_head=$(git rev-parse HEAD) || return 1
  GIT_INDEX_FILE="$index" git read-tree "$wip_head" || return 1
  GIT_INDEX_FILE="$index" git add -A . || return 1
  tree=$(GIT_INDEX_FILE="$index" git write-tree) || return 1
  [ "$tree" != "$wip_tree" ] || return 0
  next=$(git commit-tree "$tree" -p "$wip_head" -m "[skip ci] AI training ($lane): work in progress, session $stamp") || return 1
  git push --quiet --force origin "$next:refs/heads/$branch" || {
    log "cannot push the work in progress"
    return 1
  }
  wip=$next
  wip_tree=$tree
  wip_time=$(date -u +'%H:%M UTC')
}

# Rewrites the session's comment: $1 is its status line, then the work in progress and what
# attempts.md gained since the session started.
progress() {
  push_wip
  {
    printf '### Session %s\n\n' "$stamp"
    printf 'Devin (`%s`) on [`training/%s.md`](https://github.com/%s/blob/main/training/%s.md), from `main` at %s (AI generation %s). %s\n\n' \
      "$DEVIN_MODEL" "$lane" "$slug" "$lane" "$(git rev-parse --short "$base")" "$generation" "$1"
    if [ -n "$wip" ]; then
      printf '**The checkout at %s** ([its diff against `main`](https://github.com/%s/compare/%s...%s)):\n\n```\n' \
        "$wip_time" "$slug" "$base" "$wip"
      git diff --stat=100 "$base" "$wip" | tail -n 40
      printf '```\n\n'
    else
      printf 'Nothing in the checkout has changed yet.\n\n'
    fi
    if [ -f "$out/attempts.md" ] && [ "$(wc -c <"$out/attempts.md")" -gt "$attempts_mark" ]; then
      printf '**What `attempts.md` gained this session:**\n\n'
      tail -c +"$((attempts_mark + 1))" "$out/attempts.md" | tail -c 40000
      printf '\n'
    fi
  } >"$work/session.md"
  rewrite "$session_comment" "$work/session.md"
}

# One Devin session on the standing prompt, its comment kept current; sets $devin_status.
session() {
  started=$(date +%s)
  attempts_mark=0
  [ ! -f "$out/attempts.md" ] || attempts_mark=$(wc -c <"$out/attempts.md")
  wip=""
  wip_tree=$(git rev-parse "$base^{tree}")
  wip_time=""
  printf '### Session %s\n\nStarting.\n' "$stamp" >"$work/session.md"
  session_comment=$(comment "$pr" "$work/session.md") || session_comment=""
  progress "Started at $(date -u +'%H:%M UTC'); this comment is rewritten every $((progress_every / 60)) minutes while it runs."
  log "Devin session $stamp starting on training/$lane.md"
  timeout "$session_limit" devin -p --prompt-file "training/$lane.md" --model "$DEVIN_MODEL" \
    --permission-mode dangerous --respect-workspace-trust false --export "$logs/$lane-$stamp.json" </dev/null &
  devin_pid=$!
  checked=$started
  while kill -0 "$devin_pid" 2>/dev/null; do
    sleep 30
    now=$(date +%s)
    [ $((now - checked)) -ge "$progress_every" ] || continue
    checked=$now
    progress "Running for $(((now - started) / 60)) minutes; this comment is rewritten every $((progress_every / 60)) minutes while it runs."
  done
  wait "$devin_pid"
  devin_status=$?
  log "Devin session $stamp ended with status $devin_status"
}

cycle() {
  cd "$repo" || return 1
  stamp=$(date -u +%Y%m%dT%H%M%SZ)
  if [ -z "$slug" ]; then
    slug=$(gh repo view --json nameWithOwner --jq .nameWithOwner) || {
      log "gh repo view failed; waiting"
      return 1
    }
  fi
  open=$(gh pr list --head "$branch" --state open --json number,isDraft,headRefOid,mergeStateStatus,autoMergeRequest \
    --jq '.[0] // empty | "\(.number) \(.isDraft) \(.headRefOid) \(.mergeStateStatus) \(.autoMergeRequest != null)"') || {
    log "gh pr list failed; waiting"
    return 1
  }
  pr=""
  if [ -n "$open" ]; then
    # shellcheck disable=SC2086 # five words
    set -- $open
    pr=$1
    if [ "$2" = false ]; then
      watch "$1" "$3" "$4" "$5"
      return
    fi
  fi

  # 2. main, the lane's branch on it, the parent built from it, and the session's draft.
  clean_checkout || return 1
  git fetch --quiet origin main || {
    log "git fetch failed"
    return 1
  }
  git checkout --quiet -B "$branch" origin/main || {
    log "cannot reset $branch to origin/main"
    return 1
  }
  if ! cmp -s training/loop.sh "$copy"; then
    log "training/loop.sh changed on main; restarting on the new one"
    exit 0
  fi
  base=$(git rev-parse HEAD)
  generation=$(jq -er .generation crates/ai/generation.json) || {
    log "crates/ai/generation.json on main has no generation"
    return 1
  }
  if ! build_parent; then
    log "jackioh does not build on main at $base"
    return 1
  fi
  log "parent built from main at $base"

  import_records

  if ! start=$(git commit-tree "$base^{tree}" -p "$base" -m "[skip ci] AI training ($lane): session $stamp") ||
    ! git push --quiet --force origin "$start:refs/heads/$branch"; then
    log "cannot push $branch"
  fi
  title="AI gen $((generation + 1)) ($lane): in training, nothing promoted yet"
  cat >"$work/draft.md" <<EOF
The \`$lane\` training lane at work ([\`training/README.md\`](https://github.com/$slug/blob/main/training/README.md)). Devin runs on the training box one session after another, and this draft follows it:

- each session gets a comment, rewritten every $((progress_every / 60)) minutes while it runs: how long it has run, the checkout's diff against \`main\`, and what \`~/training-out/$lane/attempts.md\` gained (each attempt, its dry run's numbers and whether it was kept);
- the checkout is pushed here as one commit whenever it changes, so **Files changed** is the session's work in progress (its message tells CI to skip it);
- once a promotion passes its gate on the box, the loop lands it on \`main\`, retitles this pull request \`AI gen <N> ($lane): …\`, puts the gate's report here, marks it ready and turns on auto-merge. CI then runs every check, the training gate included;
- a promotion that fails a check comes back to draft with a comment saying which, \`attempts.md\` notes it for the next session, and the next session starts from \`main\`.

Stop the lane with \`sudo systemctl stop jackioh-train@$lane\` on the box. Closing this pull request refuses whatever is in it; the lane opens a new draft with its next session.
EOF
  if [ -n "$pr" ]; then
    gh pr edit "$pr" --title "$title" --body-file "$work/draft.md" >/dev/null || log "cannot retitle #$pr"
  elif url=$(gh pr create --draft --base main --head "$branch" --title "$title" --body-file "$work/draft.md"); then
    pr=${url##*/}
    log "opened the draft $url"
  else
    log "cannot open the draft pull request; the session runs without one"
  fi

  # 3. One Devin session on the standing prompt.
  session
  minutes=$((($(date +%s) - started) / 60))

  # 4. A promotion: land it on main and put it up.
  if ! promoted; then
    progress "Ended after $minutes minutes (Devin's exit status $devin_status) with no promotion; the next session starts from \`main\`."
    return 0
  fi
  progress "Ended after $minutes minutes (Devin's exit status $devin_status) with a promotion, \`$(git log -1 --format=%s)\`, which the loop checks again before it goes up."
  promo=$(git rev-parse HEAD)
  # One commit on main, as land expects: anything Devin committed before it is folded in.
  if [ "$(git rev-parse HEAD^)" != "$base" ]; then
    git reset --quiet --soft "$base" && git commit --quiet -C "$promo" && promo=$(git rev-parse HEAD)
  fi
  git fetch --quiet origin main || {
    log "git fetch failed before landing the promotion; it is kept as $branch-set-back-$stamp"
    git branch --quiet -f "$branch-set-back-$stamp" "$promo"
    return 1
  }
  if land "$promo" verify; then
    publish "$pr" "Promotion"
  else
    set_back "$promo" "$why" "$pr"
  fi
}

log "lane started in $repo"
while :; do
  cycle || log "the cycle stopped early; trying again after the pause"
  sleep "$pause"
done
