#!/bin/bash
# Gives back the disk that the night bot's jobs for this user can do without (README.md, Disk).
# It runs as the user whose home it cleans, never as root, from three places:
#
#   - after every job, from the runners' job-completed hook, with --job: that job's files too;
#   - every ten minutes, from the machine's disk timer, once for each agent user;
#   - at the start of a model job that finds the disk short of room (bot/harness/disk.py).
#
# Everything it deletes comes back by itself: packages from the network, logs and sessions as
# they are written again. Nothing here may fail a job, so every error is ignored.
set +e
[ "$(id -u)" != 0 ] || { echo "night-vm-clean: run it as the user whose home it cleans" >&2; exit 0; }
home="${HOME:?}"
uid="$(id -u)"
used() { df --output=pcent / 2>/dev/null | tail -1 | tr -dc 0-9; }
before="$(used)"
# Older than this many minutes: an update, a session or a job still going writes fresher files.
old=60

# --job: the job that just ended. Its files lived in RUNNER_TEMP.
if [ "${1:-}" = --job ] && [ -n "${RUNNER_TEMP:-}" ] && [ -d "$RUNNER_TEMP" ]; then
  find "$RUNNER_TEMP" -mindepth 1 -delete 2>/dev/null
fi

# The jobs this user has going, one line each (Devin and Muse have several lanes). When pgrep
# cannot tell (it exits 0 with some, 1 with none), nothing a job might be using is touched.
workers="$(pgrep -a -u "$uid" -f Runner.Worker)"
known=$?
[ "$known" -le 1 ] && known=1 || known=0
running="$(grep -c . <<<"$workers")"
# Their installs read the shared package store, so it goes only when no job but the one running
# this, if it is one, is left.
mine=0
[ -n "${RUNNER_TEMP:-}" ] && mine=1
if [ "$known" = 1 ] && [ "$running" -le "$mine" ]; then
  rm -rf "$home/.local/share/pnpm/store" "$home/.cache/pnpm" "$home/.npm/_cacache" \
    "$home/.cache/Cypress" 2>/dev/null
fi

for runner in "$home"/actions-runner*/; do
  [ -d "$runner" ] || continue
  # The model job's worktrees lived in RUNNER_TEMP. Forget the ones that are gone, or the next
  # checkout cannot reset a branch one still claims ("used by worktree at ...") and clones the
  # repository afresh. A worktree still on disk is kept.
  for repo in "$runner"_work/*/*/; do
    [ -d "$repo/.git" ] && git -C "$repo" worktree prune 2>/dev/null
  done
  # A job that died before its hook ran (the machine stopped, the runner was killed) left its
  # files in its runner's temp directory, which the runner empties only when it next takes a job:
  # a lane that is idle for days keeps them. A runner with no job going keeps none.
  if [ "$known" = 1 ] && [ -d "${runner}_work/_temp" ] && ! grep -qF "$runner" <<<"$workers"; then
    find "${runner}_work/_temp" -mindepth 1 -maxdepth 1 -mmin +"$old" -exec rm -rf {} + 2>/dev/null
  fi
  # A runner that updated itself keeps the version it replaced (bin.<version> and
  # externals.<version>, about 650 MB) beside the one `bin` and `externals` link to, and its
  # download in _work/_update. An update in progress writes fresh ones, so only old ones go.
  for kind in bin externals; do
    [ -L "$runner$kind" ] || continue
    current="$(readlink -f "$runner$kind")"
    for version in "$runner$kind".*; do
      if [ ! -d "$version" ] || [ -L "$version" ]; then continue; fi
      [ "$(readlink -f "$version")" = "$current" ] && continue
      [ -n "$(find "$version" -maxdepth 0 -mmin +"$old")" ] && rm -rf "$version"
    done
  done
  if [ -d "${runner}_work/_update" ] && [ -n "$(find "${runner}_work/_update" -maxdepth 0 -mmin +"$old")" ]; then
    rm -rf "${runner}_work/_update"
  fi
  # The runner's own logs, a few per job, which it keeps 30 days: two days are plenty.
  find "${runner}_diag" -maxdepth 1 -type f -name '*.log' -mmin +2880 -delete 2>/dev/null
done

# What this user's jobs left in /tmp, once older than any job runs (job_budget_minutes, 330): a
# younger one may be another lane's, still in use. A job on the machine keeps its temporary files
# in RUNNER_TEMP (bot-night.yml), so little lands here. (NIGHT_VM_TMP is for the tests.)
find "${NIGHT_VM_TMP:-/tmp}" -mindepth 1 -maxdepth 1 -user "$uid" -mmin +360 -exec rm -rf {} + 2>/dev/null
if [ -d "$home/.codex/sessions" ]; then
  find "$home/.codex/sessions" -type f -mtime +7 -delete 2>/dev/null
fi
# Muse keeps every session's log, a few hundred MB a day; the bot keeps its own transcripts. A
# session untouched for 8 hours has ended (a call runs at most 150 minutes).
muse_sessions="$home/.local/share/muse/sessions"
if [ -d "$muse_sessions" ]; then
  find "$muse_sessions" -mindepth 4 -maxdepth 4 -type d -mmin +480 -exec rm -rf {} + 2>/dev/null
  find "$muse_sessions/.msp-view-v1" -mindepth 1 -maxdepth 1 -type d -mmin +480 \
    -exec rm -rf {} + 2>/dev/null
  find "$muse_sessions" -mindepth 1 -type d -empty -delete 2>/dev/null
fi
# Devin keeps every session in one SQLite file, about 700 MB a day; the bot never resumes one
# (`devin -p` with `--export`). An open database is not deleted under a running session, so it
# goes only when Devin has no job going; its login is credentials.toml, beside it, and stays.
devin_cli="$home/.local/share/devin/cli"
if [ -d "$devin_cli" ]; then
  if [ "$known" = 1 ] && [ "$running" -le "$mine" ]; then
    rm -f "$devin_cli/sessions.db" "$devin_cli/sessions.db-wal" "$devin_cli/sessions.db-shm"
  fi
  find "$devin_cli/logs" "$devin_cli/summaries" -type f -mmin +480 -delete 2>/dev/null
fi
echo "night-vm-clean: $(id -un): disk ${before:-?}% full before, $(used)% after"
exit 0
