#!/bin/bash
# Registers each machine subscription's runner with GitHub and runs it as a service under that
# subscription's own user (README.md). The subscriptions are the providers.json entries whose
# runs_on is night-vm-<id>; each runner is named and labelled with its runs_on and nothing else,
# so only that subscription's jobs land on it. Run from the repository's root, signed in to gh
# as a repository admin and to the AWS CLI:
#
#   bot/machine/register-runners.sh OWNER/REPO
#
# Safe to run again: a runner already registered is only restarted.
set -euo pipefail
repo="${1:?OWNER/REPO, the repository the runners serve}"
here="$(cd "$(dirname "$0")" && pwd)"
labels="$(jq -r '.providers | to_entries[] | select(.value.runs_on // "" | startswith("night-vm-"))
  | "\(.key)=\(.value.runs_on)"' .harness/providers.json)"
[ -n "$labels" ] || { echo "no provider in .harness/providers.json runs on night-vm-*" >&2; exit 1; }
token="$(gh api -X POST "repos/$repo/actions/runners/registration-token" --jq .token)"

remote="$(mktemp)"
trap 'rm -f "$remote"' EXIT
cat > "$remote" <<'EOF'
#!/bin/bash
# On the machine, as root: $1 the repository, $2 the registration token, then id=label pairs.
set -uo pipefail
date +%s > /run/night-vm-last-busy
repo="$1"; token="$2"; shift 2
failed=0
for pair in "$@"; do
  id="${pair%%=*}"; label="${pair#*=}"; user="agent-$id"; dir="/home/$user/actions-runner"
  if [ ! -x "$dir/config.sh" ]; then
    echo "$label: no runner unpacked for $user; run setup.sh with $id first"; failed=1; continue
  fi
  cd "$dir" || continue
  if [ ! -f .runner ]; then
    if ! out="$(sudo -u "$user" ./config.sh --unattended --url "https://github.com/$repo" \
        --token "$token" --name "$label" --labels "$label" --no-default-labels --work _work \
        --replace 2>&1)"; then
      echo "$label: registration failed:"; echo "$out" | grep -iv token | tail -4; failed=1
      continue
    fi
  fi
  [ -f .service ] || ./svc.sh install "$user" >/dev/null
  ./svc.sh stop >/dev/null 2>&1; ./svc.sh start >/dev/null
  echo "$label: $(systemctl is-active "$(cat .service)") as $user"
done
exit "$failed"
EOF
# shellcheck disable=SC2086  # one argument per id=label pair
"$here/on-machine.sh" "$remote" "$repo" "$token" $labels
echo
gh api "repos/$repo/actions/runners" \
  --jq '.runners[] | "\(.name)  \(.status)  \([.labels[].name] | join(","))"'
