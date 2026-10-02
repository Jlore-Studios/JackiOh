#!/bin/bash
# Sets up the night bot's machine (README.md): Ubuntu 24.04 on x86_64, run as root. Idempotent;
# run it again to update the CLIs or to add a subscription. Arguments: the ids of the
# subscriptions that run there (providers.json entries whose runs_on is night-vm-<id>):
#
#   bash setup.sh claude-1 claude-2 claude-3 claude-4 gpt agy muse
#
# Each subscription gets a Linux user of its own, agent-<id>, with a home no other user can read,
# where its CLI login lives and where its GitHub runner (unpacked here, registered by
# register-runners.sh) runs its jobs. No agent user has sudo or Docker.
set -euo pipefail
[ "$(id -u)" = 0 ] || { echo "run as root" >&2; exit 1; }
[ "$#" -gt 0 ] || set -- claude-1 claude-2 claude-3 claude-4 gpt agy muse
export DEBIAN_FRONTEND=noninteractive HOME=/root
date +%s > /run/night-vm-last-busy  # a long setup is not idle time

# Swap: three jobs at once on 8 GB, each running the repo's checks.
if [ ! -f /swapfile ]; then
  fallocate -l 8G /swapfile && chmod 600 /swapfile && mkswap /swapfile >/dev/null && swapon /swapfile
  echo '/swapfile none swap sw 0 0' >> /etc/fstab
fi

apt-get update -qq
apt-get install -y -qq git curl unzip zip jq build-essential ca-certificates gnupg python3 \
  python3-venv python3-pip bubblewrap util-linux >/dev/null
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

# After every job, as the runner's user: give back the disk the job used. The repo's packages
# come back from the network next time; the checkout and the logins stay. The runner takes a hook
# only by its extension (.sh, .ps1 or .js), and fails the job's last step otherwise.
rm -f /usr/local/bin/night-vm-job-done
cat > /usr/local/bin/night-vm-job-done.sh <<'EOF'
#!/bin/bash
# The runner runs this with `bash -e`; nothing here may fail the job, so errors are ignored.
set +e
if [ -n "${RUNNER_TEMP:-}" ] && [ -d "$RUNNER_TEMP" ]; then
  find "$RUNNER_TEMP" -mindepth 1 -delete 2>/dev/null
fi
rm -rf "$HOME/.local/share/pnpm/store" "$HOME/.cache/pnpm" "$HOME/.npm/_cacache" 2>/dev/null
if [ -d "$HOME/.codex/sessions" ]; then
  find "$HOME/.codex/sessions" -type f -mtime +7 -delete 2>/dev/null
fi
exit 0
EOF
chmod 755 /usr/local/bin/night-vm-job-done.sh

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
systemctl daemon-reload
systemctl enable --now night-vm-idle-stop.timer >/dev/null 2>&1
apt-get clean

for cli in claude codex agy muse; do printf '%-7s %s\n' "$cli" "$($cli --version 2>&1 | head -1)"; done
echo "users: $(printf 'agent-%s ' "$@")"
df -h / | awk 'NR==2 {print "disk: " $4 " free of " $2}'
