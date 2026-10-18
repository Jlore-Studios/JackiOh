#!/bin/bash
# One ephemeral runner: JIT_CONFIG (from the supervisor, single use) registers it, it takes one
# job, and the container exits. With no JIT_CONFIG, runs the given command instead, so the same
# image does the one-time logins (login.ps1).
set -euo pipefail
mkdir -p "$HOME"
if [ -z "${JIT_CONFIG:-}" ]; then
  exec "${@:-bash}"
fi
# The checks' cargo builds use as many jobs as the container may have CPUs, not the VM's count.
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-$(nproc)}"
cfg="$JIT_CONFIG"
unset JIT_CONFIG
exec ./run.sh --jitconfig "$cfg"
