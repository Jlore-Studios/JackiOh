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
set -euo pipefail
repo="${1:?OWNER/REPO, the repository the runners serve}"
here="$(cd "$(dirname "$0")" && pwd)"
labels="$(jq -r '.providers | to_entries[] | select(.value.runs_on // "" | startswith("night-vm-"))
  | "\(.key)=\(.value.runs_on)=\(.value.lanes // 1)"' .harness/providers.json)"
[ -n "$labels" ] || { echo "no provider in .harness/providers.json runs on night-vm-*" >&2; exit 1; }
token="$(gh api -X POST "repos/$repo/actions/runners/registration-token" --jq .token)"

# A directory of its own: on-machine.sh packs the whole directory the script sits in, and the
# shared temporary directory (macOS's $TMPDIR, /tmp) holds sockets and folders tar cannot read.
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
remote="$scratch/register.sh"
cat > "$remote" <<'EOF'
#!/bin/bash
# On the machine, as root: $1 the repository, $2 the registration token, then id=label=lanes.
set -uo pipefail
date +%s > /run/night-vm-last-busy
repo="$1"; token="$2"; shift 2
failed=0
for entry in "$@"; do
  id="${entry%%=*}"; rest="${entry#*=}"; label="${rest%%=*}"; lanes="${rest#*=}"
  user="agent-$id"; first="/home/$user/actions-runner"
  if [ ! -x "$first/config.sh" ]; then
    echo "$label: no runner unpacked for $user; run setup.sh with $id first"; failed=1; continue
  fi
  for lane in $(seq 1 "$lanes"); do
    dir="$first"; name="$label"
    if [ "$lane" -gt 1 ]; then
      dir="$first-$lane"; name="$label-$lane"
      if [ ! -x "$dir/config.sh" ]; then  # a fresh copy of the unpacked runner, no registration
        sudo -u "$user" mkdir -p "$dir"
        (cd "$first" && sudo -u "$user" cp -a bin externals ./*.sh ./*.template .env .path "$dir/")
      fi
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
"$here/on-machine.sh" "$remote" "$repo" "$token" $labels
echo
gh api "repos/$repo/actions/runners" \
  --jq '.runners[] | "\(.name)  \(.status)  \([.labels[].name] | join(","))"'
