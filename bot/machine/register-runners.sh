#!/bin/bash
# Registers each machine subscription's runners with GitHub and runs them as services under that
# subscription's own user (README.md). The subscriptions are the providers.json entries whose
# runs_on is night-vm-<id>; each gets one runner per lane (`lanes`, default 1), all labelled with
# its runs_on and nothing else, so only that subscription's jobs land on them. The first is named
# after the label and lives in ~/actions-runner; a second is <label>-2 in ~/actions-runner-2. Run from the repository's root, signed in to gh
# as a repository admin and to the AWS CLI:
#
#   bot/machine/register-runners.sh OWNER/REPO
#
# Safe to run again: a runner already registered and running is left alone, job and all, except
# that one without Rust on its PATH (its .path, which config.sh writes from sudo's PATH) gets it
# and restarts once it holds no job.
#
# Two boxes (#1677: the night box and the Muse box) split the subscriptions between them. Name the
# ones this box carries, each with the runners it holds here (`id=n`; `id` alone takes its
# `lanes`), and give the second box a BOX suffix, which ends each of its runners' names: GitHub
# allows one runner of a name in a repository, and `--replace` takes a name over from another box.
# A box keeps only the runners it is given: one past its count (or every one, at `id=0`) is
# unregistered and its service removed, unless it holds a job, which a run once it ends removes.
#
#   bot/machine/register-runners.sh OWNER/REPO gpt agy devin muse=0 muse:fill
#   TAG=jackioh-train-box BOX=b2 bot/machine/register-runners.sh OWNER/REPO muse=2
#
# `id:fill` is a subscription's fill-in runners (providers.json `fill_in`): labelled with the
# fill-in's runs_on, in ~/actions-runner-fill (-2 …), one per lane of the subscriptions it fills
# unless given a count (`muse:fill=1`). Muse's two are on the night box, in gpt's and agy's lanes.
set -euo pipefail
repo="${1:?OWNER/REPO, the repository the runners serve}"
shift
here="$(cd "$(dirname "$0")" && pwd)"
box="${BOX:-}"
[[ "$box" =~ ^[a-z0-9]*$ ]] || { echo "BOX=$box: lower-case letters and digits only" >&2; exit 2; }
all="$(jq -r '.providers as $all | $all | to_entries[]
  | select(.value.runs_on // "" | startswith("night-vm-"))
  | "\(.key)=\(.value.runs_on)=\(.value.lanes // 1)",
    (select(.value.fill_in) | "\(.key):fill=\(.value.fill_in.runs_on)=\(
      [.value.fill_in.for[] | $all[.] | select(.runs_on // "" | startswith("night-vm-"))
       | .lanes // 1] | add // 0)")' .harness/providers.json)"
[ -n "$all" ] || { echo "no provider in .harness/providers.json runs on night-vm-*" >&2; exit 1; }
if [ "$#" -eq 0 ]; then
  labels="$all"
else
  labels=""
  for arg in "$@"; do
    id="${arg%%=*}"
    line="$(printf '%s\n' "$all" | grep -m1 "^$id=" || true)"
    [ -n "$line" ] || { echo "$id: no provider in .harness/providers.json runs it on night-vm-*" >&2; exit 2; }
    lanes="${line##*=}"
    [ "$arg" = "$id" ] || lanes="${arg#*=}"
    [[ "$lanes" =~ ^[0-9]+$ ]] || { echo "$arg: the runners here must be a number" >&2; exit 2; }
    labels="$labels ${line%=*}=$lanes"
  done
fi
token="$(gh api -X POST "repos/$repo/actions/runners/registration-token" --jq .token)"
remove_token="$(gh api -X POST "repos/$repo/actions/runners/remove-token" --jq .token)"

# A directory of its own: on-machine.sh packs the whole directory the script sits in, and the
# shared temporary directory (macOS's $TMPDIR, /tmp) holds sockets and folders tar cannot read.
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
remote="$scratch/register.sh"
cat > "$remote" <<'EOF'
#!/bin/bash
# On the machine, as root: $1 the repository, $2 the registration token, $3 the removal token,
# $4 the box's name suffix (empty on the night box), then id=label=lanes (id:fill for a fill-in's
# runners, which live in ~/actions-runner-fill, -2 …, copies of the unpacked runner).
set -uo pipefail
date +%s > /run/night-vm-last-busy
repo="$1"; token="$2"; remove_token="$3"; box="$4"; shift 4
failed=0
for entry in "$@"; do
  id="${entry%%=*}"; rest="${entry#*=}"; label="${rest%%=*}"; lanes="${rest#*=}"
  kind="${id#*:}"; [ "$kind" = "$id" ] && kind=""; id="${id%%:*}"
  user="agent-$id"; first="/home/$user/actions-runner"; base="$first${kind:+-$kind}"
  if [ ! -x "$first/config.sh" ]; then
    echo "$label: no runner unpacked for $user; run setup.sh with $id first"; failed=1; continue
  fi
  # Past this box's count: unregistered and gone, unless it holds a job (#1677). A lane is the
  # base directory or base-<n>; ~/actions-runner-fill is never a lane of ~/actions-runner.
  for dir in "$base" "$base"-*; do
    lane="${dir#"$base"}"; lane="${lane#-}"; lane="${lane:-1}"
    [[ "$lane" =~ ^[0-9]+$ ]] && [ "$lane" -gt "$lanes" ] && [ -f "$dir/.runner" ] || continue
    gone="$(jq -r '.agentName // empty' "$dir/.runner" 2>/dev/null || true)"; gone="${gone:-$dir}"
    if pgrep -f "$dir/bin/Runner.Worker" >/dev/null; then
      echo "$gone: busy with a job, so still registered; run this again once it ends"; continue
    fi
    if [ -f "$dir/.service" ]; then
      (cd "$dir" && ./svc.sh stop >/dev/null 2>&1; ./svc.sh uninstall >/dev/null 2>&1)
    fi
    if ! out="$(cd "$dir" && sudo -u "$user" ./config.sh remove --token "$remove_token" 2>&1)"; then
      echo "$gone: removal failed:"; echo "$out" | grep -iv token | tail -4; failed=1; continue
    fi
    [ "$dir" = "$first" ] || rm -rf "$dir"  # the unpacked runner stays, the source of every copy
    echo "$gone: removed from this box"
  done
  for lane in $(seq 1 "$lanes"); do
    dir="$base"; name="$label${box:+-$box}"
    if [ "$lane" -gt 1 ]; then
      dir="$base-$lane"; name="$label-$lane${box:+-$box}"
    fi
    if [ "$dir" != "$first" ] && [ ! -x "$dir/config.sh" ]; then
      # A fresh copy of the unpacked runner, no registration.
      sudo -u "$user" mkdir -p "$dir"
      (cd "$first" && sudo -u "$user" cp -a bin externals ./*.sh ./*.template .env .path "$dir/")
    fi
    cd "$dir" || continue
    if [ ! -f .runner ]; then
      if ! out="$(sudo -u "$user" ./config.sh --unattended --url "https://github.com/$repo" \
          --token "$token" --name "$name" --labels "$label" --no-default-labels --work _work \
          --replace 2>&1)"; then
        echo "$name: registration failed:"; echo "$out" | grep -iv token | tail -4; failed=1
        continue
      fi
    fi
    [ -f .service ] || ./svc.sh install "$user" >/dev/null
    service="$(cat .service)"
    # config.sh writes .path from the PATH it ran with, sudo's, which has no Rust: put the user's
    # Rust first again, as setup.sh does, or every job on this lane fails at `rustup`. A runner
    # reads .path only when it starts, so a running one restarts with it, unless this lane holds a
    # job, which a restart would kill: that one keeps its old .path for the next run of this.
    path="/home/$user/.cargo/bin:/usr/local/bin:/usr/bin:/bin:/usr/local/sbin:/usr/sbin:/sbin"
    fixed=""
    if [ "$(cat .path 2>/dev/null)" != "$path" ]; then
      if systemctl is-active --quiet "$service" && pgrep -f "$dir/bin/Runner.Worker" >/dev/null; then
        echo "$name: busy with a job, and no Rust on its PATH yet; run this again once it ends"
        continue
      fi
      echo "$path" | sudo -u "$user" tee .path >/dev/null
      systemctl is-active --quiet "$service" && ./svc.sh stop >/dev/null
      fixed=", Rust on its PATH now"
    fi
    # Started only if it isn't running: a restart would kill the job a running one holds.
    systemctl is-active --quiet "$service" || ./svc.sh start >/dev/null
    echo "$name: $(systemctl is-active "$service") as $user$fixed"
  done
done
exit "$failed"
EOF
# shellcheck disable=SC2086  # one argument per id=label=lanes entry
"$here/on-machine.sh" "$remote" "$repo" "$token" "$remove_token" "$box" $labels
echo
gh api "repos/$repo/actions/runners" \
  --jq '.runners[] | "\(.name)  \(.status)  \([.labels[].name] | join(","))"'
