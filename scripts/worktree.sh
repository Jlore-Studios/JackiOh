#!/usr/bin/env bash
# Create a ready-to-work git worktree for parallel development.
#
#   scripts/worktree.sh <name> [base-ref]
#
# A fresh worktree has no `node_modules`, so nothing in the web or e2e can be typechecked, linted or
# tested until dependencies are linked. pnpm's store is content-addressed and hardlinks, so the
# install is about a second rather than a full download. cargo builds each worktree into its own
# `target/` (the first build there is a full one), so an agent's engine edits are picked up by its
# own card tests, and each worktree verifies only its own changes instead of whatever another
# worktree happened to be half-way through writing.
#
# Worth knowing before reaching for one:
#   - GOOD for work that mutates files and needs its own verification (card scripts, a fuzz
#     harness, a migration sweep, anything where two agents would otherwise race on one file).
#   - BAD for the shared surfaces — the next free ruling number in spec/rulings/,
#     crates/engine/src/wire/events.rs, script.rs, state.rs, effects/mod.rs. Several isolated copies
#     each adding a note or a field conflict on merge, and one owner editing in place is strictly
#     cheaper.
#   - BAD for read-only auditing, which wants the current tree, not a snapshot.

set -euo pipefail

name="${1:-}"
base="${2:-HEAD}"

if [[ -z "$name" ]]; then
  echo "usage: scripts/worktree.sh <name> [base-ref]" >&2
  exit 2
fi

root="$(git rev-parse --show-toplevel)"
dir="$root/../jackioh-wt/$name"

mkdir -p "$(dirname "$dir")"

if git show-ref --verify --quiet "refs/heads/wt/$name"; then
  git worktree add "$dir" "wt/$name"
else
  git worktree add -b "wt/$name" "$dir" "$base"
fi

cd "$dir"
pnpm install --prefer-offline --silent

echo
echo "worktree ready: $dir (branch wt/$name)"
echo "  verify with:  cd '$dir' && cargo test -p jackioh-engine --features testkit --test rules"
echo "  merge back:   git -C '$root' merge wt/$name"
echo "  remove with:  git -C '$root' worktree remove '$dir'"
