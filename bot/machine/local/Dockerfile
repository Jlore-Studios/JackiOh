# The local runner's image (bot/machine/README.md, The local runner): the tools bot/machine/setup.sh
# puts on the night box (Node 24 and pnpm through corepack, Python 3.12, Rust 1.97.0, a C++ compiler,
# the claude, codex, agy, muse and devin CLIs) and the GitHub Actions runner, in one image. Each
# container is one ephemeral runner: it takes one job and exits. A night box subscription's login
# lives in its own volume, mounted at /home/runner; a self-play runner has none. Built from this directory:
#   docker build -t jackioh-pc-runner:latest bot/machine/local
FROM ubuntu:24.04
SHELL ["/bin/bash", "-o", "pipefail", "-c"]

ARG RUST_TOOLCHAIN=1.97.0
ARG NODE_MAJOR=24
ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:/usr/local/bin:/usr/bin:/bin:/usr/local/sbin:/usr/sbin:/sbin

RUN apt-get update -qq \
 && apt-get install -y -qq --no-install-recommends git curl unzip zip jq build-essential \
      ca-certificates gnupg python3 python3-venv python3-pip bubblewrap procps tini \
 && curl -fsSL "https://deb.nodesource.com/setup_${NODE_MAJOR}.x" | bash - >/dev/null \
 && apt-get install -y -qq nodejs >/dev/null \
 && corepack enable \
 && rm -rf /var/lib/apt/lists/*

# The model CLIs, installed system-wide as on the night box.
RUN npm install -g --loglevel=error @anthropic-ai/claude-code@latest @openai/codex@latest \
 && curl -fsSL https://antigravity.google/cli/install.sh -o /tmp/agy.sh \
 && HOME=/root bash /tmp/agy.sh >/dev/null 2>&1 && install -m 755 /root/.local/bin/agy /usr/local/bin/agy \
 && mkdir -p /usr/local/lib/muse && curl -fsSL https://dev.meta.ai/install.sh -o /tmp/muse.sh \
 && MUSE_INSTALL_DIR=/usr/local/lib/muse MUSE_NO_AUTO_UPDATE=1 bash /tmp/muse.sh >/dev/null 2>&1 \
 && ln -sf /usr/local/lib/muse/muse /usr/local/bin/muse && chmod -R a+rX /usr/local/lib/muse \
 && manifest="$(curl -fsSL https://static.devin.ai/cli/current/manifest.json)" \
 && curl -fsSL -o /tmp/devin.tgz "$(jq -r '.platforms["x86_64-unknown-linux"].url' <<<"$manifest")" \
 && echo "$(jq -r '.platforms["x86_64-unknown-linux"].sha256' <<<"$manifest")  /tmp/devin.tgz" | sha256sum -c --quiet - \
 && mkdir -p /usr/local/lib/devin && tar xzf /tmp/devin.tgz -C /usr/local/lib/devin \
 && echo curl-bash > /usr/local/lib/devin/distribution \
 && ln -sf /usr/local/lib/devin/bin/devin /usr/local/bin/devin && chmod -R a+rX /usr/local/lib/devin \
 && rm -f /tmp/*.sh /tmp/devin.tgz
# The job user: no sudo, no Docker socket, like the night box's agent-<id> users.
RUN useradd -m -u 1001 -s /bin/bash runner \
 && mkdir -p /opt/rustup /opt/cargo /actions-runner && chown runner:runner /opt/rustup /opt/cargo /actions-runner

# The GitHub Actions runner (latest at build time; rebuild the image to update it).
RUN version="$(curl -fsSL https://api.github.com/repos/actions/runner/releases/latest | jq -r .tag_name)" \
 && curl -fsSL "https://github.com/actions/runner/releases/download/${version}/actions-runner-linux-x64-${version#v}.tar.gz" \
    | tar xz -C /actions-runner \
 && /actions-runner/bin/installdependencies.sh >/dev/null \
 && chown -R runner:runner /actions-runner && rm -rf /var/lib/apt/lists/*

USER runner
RUN curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path --profile minimal --default-toolchain "${RUST_TOOLCHAIN}" \
        --component rustfmt,clippy --target wasm32-unknown-unknown \
 && rustc --version

# What the night box's runner .env sets, plus: never see a GPU.
ENV GEMINI_FORCE_FILE_STORAGE=true \
    TBH_CREDENTIAL_BACKEND=file \
    MUSE_NO_AUTO_UPDATE=1 \
    COREPACK_ENABLE_DOWNLOAD_PROMPT=0 \
    CYPRESS_INSTALL_BINARY=0 \
    NVIDIA_VISIBLE_DEVICES=void \
    CUDA_VISIBLE_DEVICES="" \
    HOME=/home/runner

COPY --chown=runner:runner entrypoint.sh /entrypoint.sh
WORKDIR /actions-runner
ENTRYPOINT ["/usr/bin/tini", "--", "/entrypoint.sh"]
