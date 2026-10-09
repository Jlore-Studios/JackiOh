#!/bin/bash
# Sets up one of the bot's two boxes (README.md): Ubuntu 24.04 on x86_64, run as root. Idempotent;
# run it again to update the CLIs, the Rust toolchain or to add a subscription.
#
# The night box: the ids of the subscriptions that run there (providers.json entries whose
# runs_on is night-vm-<id>):
#
#   bash setup.sh gpt agy muse devin
#
# Each subscription gets a Linux user of its own, agent-<id>, with a home no other user can read,
# where its CLI login lives, its Rust toolchain for the light checks, and its GitHub runner
# (unpacked here, registered by register-runners.sh) runs its jobs. No agent user has sudo or
# Docker.
#
# The training box (training/README.md), with --training and nothing else:
#
#   bash setup.sh --training
#
# Two users, agent-train-improve and agent-train-unban, each with its own checkout, Rust, Devin
# and gh, and the systemd service jackioh-train@<lane> that runs training/loop.sh <lane> forever.
# The box is always on: no idle stop, no runner, no starter. Each user's Devin login and GitHub
# token are a person's to put in once (README.md, The training box). TRAIN_REPO (OWNER/REPO) is the
# repository the lanes clone, push to and open pull requests on.
set -euo pipefail
[ "$(id -u)" = 0 ] || { echo "run as root" >&2; exit 1; }
training=""
if [ "${1:-}" = --training ]; then
  training=1
  shift
  [ "$#" = 0 ] || { echo "setup.sh --training takes no subscription ids" >&2; exit 2; }
fi
# The Claude accounts log in from secrets and run on GitHub's runners, so none has a user here.
[ -n "$training" ] || [ "$#" -gt 0 ] || set -- gpt agy muse devin
export DEBIAN_FRONTEND=noninteractive HOME=/root
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# The version rust-toolchain.toml pins (with its components and target), installed for every user
# that runs the repository's checks, so cargo in a checkout never downloads a toolchain mid-job.
# Change it with rust-toolchain.toml, then run this again on both boxes.
RUST_TOOLCHAIN=1.97.0
TRAIN_REPO="${TRAIN_REPO:-Jlore-Studios/JackiOh}"
TRAIN_LANES="improve unban"
[ -n "$training" ] || date +%s > /run/night-vm-last-busy  # a long setup is not idle time

# Swap: three jobs at once on 8 GB, each running the repo's checks; on the training box, room for
# both lanes' release builds beside their games.
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

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
chmod 755 "$work"  # the agent users read the installers below from here

# rustup's installer, run once per user by install_rust.
curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs -o "$work/rustup-init.sh"
chmod 644 "$work/rustup-init.sh"

# Rust for one user, in that user's home: rustup, then RUST_TOOLCHAIN on the minimal profile with
# rustfmt and clippy (the light checks: `cargo fmt --check` and the `cargo jackioh` checks) and the
# wasm32 target rust-toolchain.toml lists. It runs from the user's home, which only that user (and
# root) may enter. Run again, it changes nothing.
install_rust() {
  local user="$1" home="/home/$1" out
  if ! out="$(
      cd "$home" || exit 1
      if [ ! -x "$home/.cargo/bin/rustup" ]; then
        sudo -u "$user" -H sh "$work/rustup-init.sh" -y --no-modify-path --profile minimal \
          --default-toolchain none 2>&1 || exit 1
      fi
      sudo -u "$user" -H "$home/.cargo/bin/rustup" toolchain install "$RUST_TOOLCHAIN" \
        --profile minimal --component rustfmt,clippy --target wasm32-unknown-unknown 2>&1 || exit 1
      sudo -u "$user" -H "$home/.cargo/bin/rustup" default "$RUST_TOOLCHAIN" 2>&1
    )"; then
    echo "$user: Rust $RUST_TOOLCHAIN did not install:" >&2
    echo "$out" | tail -20 >&2
    exit 1
  fi
}

# Devin's CLI: the steps of cli.devin.ai/install.sh (its manifest, the bundle, the bundle's
# checksum), into a shared directory instead of root's home, and without the per-user
# `devin setup` it ends with. Each user's own login stays in that user's home.
install_devin() {
  local manifest version dir
  manifest="$(curl -fsSL https://static.devin.ai/cli/current/manifest.json)"
  version="$(jq -r .version <<<"$manifest")"
  dir="/usr/local/lib/devin/$version"
  if [ ! -x "$dir/bin/devin" ]; then
    curl -fsSL -o "$work/devin.tgz" \
      "$(jq -r '.platforms["x86_64-unknown-linux"].url' <<<"$manifest")"
    echo "$(jq -r '.platforms["x86_64-unknown-linux"].sha256' <<<"$manifest")  $work/devin.tgz" \
      | sha256sum -c --quiet -
    rm -rf "$dir.tmp" && mkdir -p "$dir.tmp"
    tar xzf "$work/devin.tgz" -C "$dir.tmp"
    rm -rf "$dir" && mv "$dir.tmp" "$dir"
  fi
  echo curl-bash > "$dir/distribution"
  ln -sfn "$version" /usr/local/lib/devin/current
  ln -sf /usr/local/lib/devin/current/bin/devin /usr/local/bin/devin
  chmod -R a+rX /usr/local/lib/devin
}

# Node 24 (the repo's .nvmrc), and pnpm through corepack (package.json pins the version). On the
# training box, loop.sh and Devin run the web's tests that play the AI with them before a promotion
# goes up (training/README.md).
if ! node --version 2>/dev/null | grep -q '^v24\.'; then
  curl -fsSL https://deb.nodesource.com/setup_24.x | bash - >/dev/null
  apt-get install -y -qq nodejs >/dev/null
fi
corepack enable

if [ -n "$training" ]; then
  # GitHub's CLI from GitHub's own apt repository (cli.github.com): loop.sh lists, opens and
  # auto-merges the lanes' pull requests with it, and git pushes through it.
  if [ ! -f /etc/apt/sources.list.d/github-cli.list ]; then
    install -d -m 755 /etc/apt/keyrings
    curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg \
      -o /etc/apt/keyrings/githubcli-archive-keyring.gpg
    chmod go+r /etc/apt/keyrings/githubcli-archive-keyring.gpg
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/githubcli-archive-keyring.gpg] https://cli.github.com/packages stable main" \
      > /etc/apt/sources.list.d/github-cli.list
    apt-get update -qq
  fi
  apt-get install -y -qq gh >/dev/null
  install_devin

  # The box is always on. A box set up before as a night box (or by the old `setup.sh
  # devin-train`) has the idle stop: it goes.
  systemctl disable --now night-vm-idle-stop.timer >/dev/null 2>&1 || true
  rm -f /etc/systemd/system/night-vm-idle-stop.timer /etc/systemd/system/night-vm-idle-stop.service

  for lane in $TRAIN_LANES; do
    user="agent-train-$lane"
    home="/home/$user"
    id "$user" >/dev/null 2>&1 || useradd -m -s /bin/bash -K UMASK=077 "$user"
    chmod 700 "$home"
    install_rust "$user"
    # The lane's own checkout, which loop.sh resets to main every cycle (so its target/ stays and
    # each build after the first is incremental).
    if [ ! -d "$home/JackiOh/.git" ]; then
      (cd "$home" && sudo -u "$user" -H git clone --quiet "https://github.com/$TRAIN_REPO.git" JackiOh)
    fi
    # git pushes through gh's login (what `gh auth setup-git` writes), and commits under a name of
    # the lane's own unless a person set one: Devin commits its promotion and loop.sh rebases it.
    (cd "$home" && sudo -u "$user" -H git config --global --replace-all \
      credential.https://github.com.helper '!/usr/bin/gh auth git-credential')
    if ! (cd "$home" && sudo -u "$user" -H git config --global user.email >/dev/null); then
      (cd "$home" && sudo -u "$user" -H git config --global user.name "JackiOh training ($lane)")
      (cd "$home" && sudo -u "$user" -H git config --global user.email "$user@jackioh-train-box")
    fi
    sudo -u "$user" mkdir -p "$home/training-out/$lane" "$home/logs"
  done

  # One lane, forever: training/loop.sh <lane> as the lane's user, from its checkout. A system
  # unit's %h is root's home whatever User= says, so the paths name the lane's home in full; the
  # loop sets JACKIOH_TRAINING_OUT to the same directory itself. RAYON_NUM_THREADS=2: two lanes'
  # games share the box's vCPUs. DEVIN_MODEL is the knob for Devin's model.
  cat > /etc/systemd/system/jackioh-train@.service <<'EOF'
[Unit]
Description=JackiOh AI training lane %i (training/README.md)
Wants=network-online.target
After=network-online.target

[Service]
User=agent-train-%i
WorkingDirectory=/home/agent-train-%i/JackiOh
ExecStart=/bin/sh /home/agent-train-%i/JackiOh/training/loop.sh %i
Restart=always
RestartSec=60
Environment=DEVIN_MODEL=swe-2-max
Environment=RAYON_NUM_THREADS=2
Environment=JACKIOH_TRAINING_OUT=/home/agent-train-%i/training-out/%i
Environment=COREPACK_ENABLE_DOWNLOAD_PROMPT=0
Environment=CYPRESS_INSTALL_BINARY=0
Environment=PATH=/home/agent-train-%i/.cargo/bin:/usr/local/bin:/usr/bin:/bin

[Install]
WantedBy=multi-user.target
EOF
  systemctl daemon-reload
  # A lane already running keeps its Devin session; it reads a changed unit at its next restart
  # (`systemctl restart jackioh-train@<lane>`).
  systemctl enable --now jackioh-train@improve jackioh-train@unban
  apt-get clean

  for lane in $TRAIN_LANES; do
    user="agent-train-$lane"
    printf '%-20s %s; rustc %s; %s\n' "jackioh-train@$lane" \
      "$(systemctl is-active "jackioh-train@$lane" || true)" \
      "$(cd "/home/$user" && sudo -u "$user" -H "/home/$user/.cargo/bin/rustc" --version 2>&1 | awk '{print $2}')" \
      "$(cd "/home/$user" && sudo -u "$user" -H gh auth status >/dev/null 2>&1 && echo "gh logged in" || echo "gh not logged in")"
  done
  printf '%-7s %s\n' devin "$(devin --version 2>&1 | head -1)" gh "$(gh --version 2>&1 | head -1)" \
    node "$(node --version 2>&1)"
  df -h / | awk 'NR==2 {print "disk: " $4 " free of " $2}'
  exit 0
fi

# Codex sandboxes commands with bubblewrap, which Ubuntu's AppArmor blocks by default.
echo 'kernel.apparmor_restrict_unprivileged_userns=0' > /etc/sysctl.d/60-codex-bwrap.conf
sysctl -q -p /etc/sysctl.d/60-codex-bwrap.conf

# The model CLIs, installed for every user. Each user's login stays in its own home.
npm install -g --loglevel=error @anthropic-ai/claude-code@latest @openai/codex@latest >/dev/null
curl -fsSL https://antigravity.google/cli/install.sh -o "$work/agy-install.sh"
bash "$work/agy-install.sh" >/dev/null 2>&1
install -m 755 /root/.local/bin/agy /usr/local/bin/agy
mkdir -p /usr/local/lib/muse
curl -fsSL https://dev.meta.ai/install.sh -o "$work/muse-install.sh"
MUSE_INSTALL_DIR=/usr/local/lib/muse MUSE_NO_AUTO_UPDATE=1 bash "$work/muse-install.sh" >/dev/null 2>&1
ln -sf /usr/local/lib/muse/muse /usr/local/bin/muse
chmod -R a+rX /usr/local/lib/muse
install_devin

# The disk's clean-up (clean.sh, beside this script: on-machine.sh ships the folder), run as each
# user: after every job by the runners' job-completed hook, and every ten minutes by the disk
# timer below. The repo's packages come back from the network next time; the checkout and the
# logins stay. The runner takes a hook only by its extension (.sh, .ps1 or .js), and fails the
# job's last step otherwise.
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
chmod 644 "$work/runner.tgz"
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
  # Rust on the user's own shell's PATH too (the runners get it from .path, below). $HOME and
  # $PATH are written as they are, for the user's shell to expand.
  # shellcheck disable=SC2016
  grep -q '.cargo/bin' "/home/$user/.profile" \
    || echo 'export PATH="$HOME/.cargo/bin:$PATH"  # Rust, from setup.sh' >> "/home/$user/.profile"
  install_rust "$user"
  dir="/home/$user/actions-runner"
  if [ ! -x "$dir/config.sh" ]; then
    sudo -u "$user" mkdir -p "$dir"
    sudo -u "$user" tar xzf "$work/runner.tgz" -C "$dir"
  fi
  if [ -z "$installed_deps" ]; then
    "$dir/bin/installdependencies.sh" >/dev/null && installed_deps=1
  fi
  # The job environment of every runner of this user (register-runners.sh copies the first one's
  # for a second lane, once): the same login settings as the user's shell, the clean-up hook, and
  # the user's Rust ahead of the system's PATH.
  for dir in "/home/$user"/actions-runner*; do
    [ -x "$dir/config.sh" ] || continue
    sudo -u "$user" tee "$dir/.env" >/dev/null <<'EOF'
LANG=C.UTF-8
GEMINI_FORCE_FILE_STORAGE=true
TBH_CREDENTIAL_BACKEND=file
MUSE_NO_AUTO_UPDATE=1
COREPACK_ENABLE_DOWNLOAD_PROMPT=0
CYPRESS_INSTALL_BINARY=0
ACTIONS_RUNNER_HOOK_JOB_COMPLETED=/usr/local/bin/night-vm-job-done.sh
EOF
    echo "/home/$user/.cargo/bin:/usr/local/bin:/usr/bin:/bin:/usr/local/sbin:/usr/sbin:/sbin" \
      | sudo -u "$user" tee "$dir/.path" >/dev/null
    # A running runner reads .env and .path only when it starts; one in the middle of a job is
    # left alone, since restarting it would kill the job (run this again later).
    if [ -f "$dir/.service" ]; then
      if pgrep -u "$user" -f Runner.Worker >/dev/null; then
        echo "${dir##*/} of $user: busy with a job; it picks up the new settings at its next restart"
      else
        (cd "$dir" && ./svc.sh stop >/dev/null && ./svc.sh start >/dev/null)
      fi
    fi
  done
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

# Memory on record (#317, owner action B): CloudWatch keeps only the CPU of an instance, so the
# CloudWatch agent sends memory and swap every 60 s to the JackiOh/NightVM namespace. It needs the
# instance role to hold CloudWatchAgentServerPolicy (README.md, Memory); without it the agent
# runs and sends nothing, and nothing else is affected.
CW=/opt/aws/amazon-cloudwatch-agent
if [ ! -x "$CW/bin/amazon-cloudwatch-agent-ctl" ]; then
  if curl -fsSL -o /tmp/cwagent.deb \
      https://amazoncloudwatch-agent.s3.amazonaws.com/ubuntu/amd64/latest/amazon-cloudwatch-agent.deb; then
    dpkg -i -E /tmp/cwagent.deb >/dev/null || echo "warning: the CloudWatch agent did not install"
  else
    echo "warning: the CloudWatch agent could not be downloaded"
  fi
  rm -f /tmp/cwagent.deb
fi
if [ -x "$CW/bin/amazon-cloudwatch-agent-ctl" ]; then
  cat > "$CW/etc/night-vm.json" <<'EOF'
{
  "agent": { "metrics_collection_interval": 60, "run_as_user": "cwagent" },
  "metrics": {
    "namespace": "JackiOh/NightVM",
    "append_dimensions": { "InstanceId": "${aws:InstanceId}" },
    "metrics_collected": {
      "mem": { "measurement": ["mem_used_percent", "mem_available"] },
      "swap": { "measurement": ["swap_used_percent"] }
    }
  }
}
EOF
  "$CW/bin/amazon-cloudwatch-agent-ctl" -a fetch-config -m ec2 -s -c "file:$CW/etc/night-vm.json" \
    >/dev/null || echo "warning: the CloudWatch agent did not start"
fi
apt-get clean

for cli in claude codex agy muse devin; do printf '%-7s %s\n' "$cli" "$($cli --version 2>&1 | head -1)"; done
for id in "$@"; do
  printf '%-7s rustc %s\n' "$id" \
    "$(cd "/home/agent-$id" && sudo -u "agent-$id" -H "/home/agent-$id/.cargo/bin/rustc" --version 2>&1 | awk '{print $2}')"
done
echo "users: $(printf 'agent-%s ' "$@")"
df -h / | awk 'NR==2 {print "disk: " $4 " free of " $2}'
