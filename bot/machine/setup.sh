#!/bin/bash
# Sets up the night bot's machine (README.md): Ubuntu 24.04 on x86_64, run as root. Idempotent;
# run it again to update the CLIs or to add a subscription. Arguments: the ids of the
# subscriptions that run there (providers.json entries whose runs_on is night-vm-<id>):
#
#   bash setup.sh gpt agy muse devin
#
# Each subscription gets a Linux user of its own, agent-<id>, with a home no other user can read,
# where its CLI login lives and where its GitHub runner (unpacked here, registered by
# register-runners.sh) runs its jobs. No agent user has sudo or Docker.
set -euo pipefail
[ "$(id -u)" = 0 ] || { echo "run as root" >&2; exit 1; }
# The Claude accounts log in from secrets and run on GitHub's runners, so none has a user here.
[ "$#" -gt 0 ] || set -- gpt agy muse devin
export DEBIAN_FRONTEND=noninteractive HOME=/root
date +%s > /run/night-vm-last-busy  # a long setup is not idle time

# Swap: three jobs at once on 8 GB, each running the repo's checks.
if [ ! -f /swapfile ]; then
  fallocate -l 8G /swapfile && chmod 600 /swapfile && mkswap /swapfile >/dev/null && swapon /swapfile
  echo '/swapfile none swap sw 0 0' >> /etc/fstab
fi

apt-get update -qq
apt-get install -y -qq git curl unzip zip jq build-essential ca-certificates gnupg python3 \
  python3-venv python3-pip bubblewrap util-linux cloud-guest-utils >/dev/null
# A disk made larger in EC2 (Modify volume) is grown into by cloud-init at the next boot; this
# grows the partition and the filesystem now, so the room counts without a restart.
root_src="$(findmnt -no SOURCE / || true)"
root_disk="$(lsblk -no PKNAME "$root_src" 2>/dev/null | head -1 || true)"
root_part="$(cat "/sys/class/block/${root_src##*/}/partition" 2>/dev/null || true)"
if [ -n "$root_disk" ] && [ -n "$root_part" ] \
   && growpart "/dev/$root_disk" "$root_part" >/dev/null 2>&1; then
  resize2fs "$root_src" >/dev/null 2>&1 || true
fi
# The system journal may otherwise take a tenth of the disk (up to 4 GB).
mkdir -p /etc/systemd/journald.conf.d
printf '[Journal]\nSystemMaxUse=200M\n' > /etc/systemd/journald.conf.d/night-vm.conf
systemctl restart systemd-journald
journalctl --vacuum-size=200M >/dev/null 2>&1 || true

# Codex sandboxes commands with bubblewrap, which Ubuntu's AppArmor blocks by default.
echo 'kernel.apparmor_restrict_unprivileged_userns=0' > /etc/sysctl.d/60-codex-bwrap.conf
sysctl -q -p /etc/sysctl.d/60-codex-bwrap.conf

# Node 24 (the repo's .nvmrc), and pnpm through corepack (package.json pins the version).
if ! node --version 2>/dev/null | grep -q '^v24\.'; then
  curl -fsSL https://deb.nodesource.com/setup_24.x | bash - >/dev/null
  apt-get install -y -qq nodejs >/dev/null
fi
corepack enable

# The model CLIs, installed for every user. Each user's login stays in its own home.
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
npm install -g --loglevel=error @anthropic-ai/claude-code@latest @openai/codex@latest >/dev/null
curl -fsSL https://antigravity.google/cli/install.sh -o "$work/agy-install.sh"
bash "$work/agy-install.sh" >/dev/null 2>&1
install -m 755 /root/.local/bin/agy /usr/local/bin/agy
mkdir -p /usr/local/lib/muse
curl -fsSL https://dev.meta.ai/install.sh -o "$work/muse-install.sh"
MUSE_INSTALL_DIR=/usr/local/lib/muse MUSE_NO_AUTO_UPDATE=1 bash "$work/muse-install.sh" >/dev/null 2>&1
ln -sf /usr/local/lib/muse/muse /usr/local/bin/muse
chmod -R a+rX /usr/local/lib/muse
# Devin's CLI: the steps of cli.devin.ai/install.sh (its manifest, the bundle, the bundle's
# checksum), into a shared directory instead of root's home, and without the per-user
# `devin setup` it ends with. Each user's own login stays in that user's home.
devin_manifest="$(curl -fsSL https://static.devin.ai/cli/current/manifest.json)"
devin_version="$(jq -r .version <<<"$devin_manifest")"
devin_dir="/usr/local/lib/devin/$devin_version"
if [ ! -x "$devin_dir/bin/devin" ]; then
  curl -fsSL -o "$work/devin.tgz" \
    "$(jq -r '.platforms["x86_64-unknown-linux"].url' <<<"$devin_manifest")"
  echo "$(jq -r '.platforms["x86_64-unknown-linux"].sha256' <<<"$devin_manifest")  $work/devin.tgz" \
    | sha256sum -c --quiet -
  rm -rf "$devin_dir.tmp" && mkdir -p "$devin_dir.tmp"
  tar xzf "$work/devin.tgz" -C "$devin_dir.tmp"
  rm -rf "$devin_dir" && mv "$devin_dir.tmp" "$devin_dir"
fi
echo curl-bash > "$devin_dir/distribution"
ln -sfn "$devin_version" /usr/local/lib/devin/current
ln -sf /usr/local/lib/devin/current/bin/devin /usr/local/bin/devin
chmod -R a+rX /usr/local/lib/devin

# The disk's clean-up (clean.sh, beside this script: on-machine.sh ships the folder), run as each
# user: after every job by the runners' job-completed hook, and every ten minutes by the disk
# timer below. The repo's packages come back from the network next time; the checkout and the
# logins stay. The runner takes a hook only by its extension (.sh, .ps1 or .js), and fails the
# job's last step otherwise.
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
install -m 755 "$here/clean.sh" /usr/local/bin/night-vm-clean.sh
rm -f /usr/local/bin/night-vm-job-done
cat > /usr/local/bin/night-vm-job-done.sh <<'EOF'
#!/bin/bash
# The runner runs this with `bash -e` after every job, as the job's user; it must not fail the job.
/usr/local/bin/night-vm-clean.sh --job || true
exit 0
EOF
chmod 755 /usr/local/bin/night-vm-job-done.sh
cat > /usr/local/bin/night-vm-disk <<'EOF'
#!/bin/bash
# Every ten minutes: each agent user cleans its own home as itself (night-vm-clean.sh), so a
# lane that sits idle still gives back what its last job left, and the system trims its journal
# and its package cache.
for home in /home/agent-*; do
  user="${home##*/}"
  id "$user" >/dev/null 2>&1 || continue
  runuser -u "$user" -- env -i HOME="$home" PATH=/usr/local/bin:/usr/bin:/bin \
    /usr/local/bin/night-vm-clean.sh | logger -t night-vm-disk
done
journalctl --vacuum-size=200M >/dev/null 2>&1
apt-get clean
exit 0
EOF
chmod 755 /usr/local/bin/night-vm-disk

# The newest GitHub Actions runner, unpacked once per user (register-runners.sh registers it).
version="$(curl -fsSL https://api.github.com/repos/actions/runner/releases/latest | jq -r .tag_name)"
version="${version#v}"
curl -fsSL -o "$work/runner.tgz" \
  "https://github.com/actions/runner/releases/download/v${version}/actions-runner-linux-x64-${version}.tar.gz"
chmod 755 "$work" && chmod 644 "$work/runner.tgz"
installed_deps=""

for id in "$@"; do
  user="agent-$id"
  id "$user" >/dev/null 2>&1 || useradd -m -s /bin/bash -K UMASK=077 "$user"
  chmod 700 "/home/$user"
  grep -q MUSE_NO_AUTO_UPDATE "/home/$user/.profile" || cat >> "/home/$user/.profile" <<'EOF'
# Logins stay in files in this home; the night bot's jobs for this subscription run as this user.
export GEMINI_FORCE_FILE_STORAGE=true
export TBH_CREDENTIAL_BACKEND=file
export MUSE_NO_AUTO_UPDATE=1
unset META_API_KEY
EOF
  dir="/home/$user/actions-runner"
  if [ ! -x "$dir/config.sh" ]; then
    sudo -u "$user" mkdir -p "$dir"
    sudo -u "$user" tar xzf "$work/runner.tgz" -C "$dir"
  fi
  if [ -z "$installed_deps" ]; then
    "$dir/bin/installdependencies.sh" >/dev/null && installed_deps=1
  fi
  # The job environment: the same login settings as the user's shell, and the clean-up hook.
  sudo -u "$user" tee "$dir/.env" >/dev/null <<'EOF'
LANG=C.UTF-8
GEMINI_FORCE_FILE_STORAGE=true
TBH_CREDENTIAL_BACKEND=file
MUSE_NO_AUTO_UPDATE=1
COREPACK_ENABLE_DOWNLOAD_PROMPT=0
CYPRESS_INSTALL_BINARY=0
ACTIONS_RUNNER_HOOK_JOB_COMPLETED=/usr/local/bin/night-vm-job-done.sh
EOF
  echo "/usr/local/bin:/usr/bin:/bin:/usr/local/sbin:/usr/sbin:/sbin" | sudo -u "$user" tee "$dir/.path" >/dev/null
  # A running runner reads .env and .path only when it starts; one in the middle of a job is left
  # alone, since restarting it would kill the job (run this again later).
  if [ -f "$dir/.service" ]; then
    if pgrep -u "$user" -f Runner.Worker >/dev/null; then
      echo "$user: busy with a job; its runner picks up the new settings at its next restart"
    else
      (cd "$dir" && ./svc.sh stop >/dev/null && ./svc.sh start >/dev/null)
    fi
  fi
done

# Power off when idle: no job running (Runner.Worker), no Session Manager session, and first-boot
# setup done. The instance's shutdown behaviour is stop, and the starter starts it again.
cat > /usr/local/bin/night-vm-idle-stop <<'EOF'
#!/bin/bash
IDLE_MINUTES=30
STAMP=/run/night-vm-last-busy
if pgrep -f Runner.Worker >/dev/null || pgrep -u ssm-user >/dev/null \
   || cloud-init status 2>/dev/null | grep -q running; then
  date +%s > "$STAMP"; exit 0
fi
[ -f "$STAMP" ] || date +%s > "$STAMP"
if [ $(( ($(date +%s) - $(cat "$STAMP")) / 60 )) -ge "$IDLE_MINUTES" ]; then
  logger -t night-vm "idle for ${IDLE_MINUTES} minutes; powering off"
  shutdown -h now
fi
EOF
chmod 755 /usr/local/bin/night-vm-idle-stop
cat > /etc/systemd/system/night-vm-idle-stop.service <<'EOF'
[Unit]
Description=Power the night machine off when no job has run for a while
[Service]
Type=oneshot
ExecStart=/usr/local/bin/night-vm-idle-stop
EOF
cat > /etc/systemd/system/night-vm-idle-stop.timer <<'EOF'
[Unit]
Description=Check every five minutes whether the night machine is idle
[Timer]
OnBootSec=5min
OnUnitActiveSec=5min
[Install]
WantedBy=timers.target
EOF
cat > /etc/systemd/system/night-vm-disk.service <<'EOF'
[Unit]
Description=Give back the disk the night bot's jobs can do without
[Service]
Type=oneshot
Nice=10
IOSchedulingClass=idle
ExecStart=/usr/local/bin/night-vm-disk
EOF
cat > /etc/systemd/system/night-vm-disk.timer <<'EOF'
[Unit]
Description=Clean the night machine's disk every ten minutes
[Timer]
OnBootSec=2min
OnUnitActiveSec=10min
[Install]
WantedBy=timers.target
EOF
systemctl daemon-reload
systemctl enable --now night-vm-idle-stop.timer night-vm-disk.timer >/dev/null 2>&1
apt-get clean

for cli in claude codex agy muse devin; do printf '%-7s %s\n' "$cli" "$($cli --version 2>&1 | head -1)"; done
echo "users: $(printf 'agent-%s ' "$@")"
df -h / | awk 'NR==2 {print "disk: " $4 " free of " $2}'
