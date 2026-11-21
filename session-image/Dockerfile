# The image every session's container starts from (`antilog runner --image`). It holds the harness
# (OpenCode, one pinned version), git and Docker for the agent. It holds no infrastructure tool:
# no OpenTofu, Terraform or cloud CLI (docs/V0.md). An environment's setup script adds what a
# project needs on top: the runner runs it in each fresh container after cloning the repos.
#
#   docker build -t antilog-session:latest session-image
#   (behind a TLS-inspecting proxy add: --secret id=ca,src=/path/to/ca.crt; apt, npm, pip and git use it)
FROM node:22-bookworm-slim

# The pinned OpenCode: the version `.claude/skills/session/runner.md` was verified against.
ARG OPENCODE_VERSION=1.19.6

# `docker.io` is the client and the daemon. The runner starts the daemon in the container only
# when the host runs it under sysbox, so the agent has Docker without `--privileged`.
# python3, pip and venv are what most repos' tooling expects of a machine.
# The apt sources are switched to https after ca-certificates is in, so `apt` works in a running
# session too: the proxy that is its only way out tunnels HTTPS and nothing else.
# Behind a TLS-inspecting proxy, pass its CA as a build secret (`--secret id=ca,src=ca.crt`):
# apt (which runs as root for this step, to read the secret), npm, pip and git trust it for this step
# only and it never lands in the image.
RUN --mount=type=secret,id=ca,required=false \
    apt_ca=""; \
    if [ -s /run/secrets/ca ]; then \
        apt_ca="-o Acquire::https::CaInfo=/run/secrets/ca -o APT::Sandbox::User=root"; \
        export NODE_EXTRA_CA_CERTS=/run/secrets/ca PIP_CERT=/run/secrets/ca GIT_SSL_CAINFO=/run/secrets/ca; \
    fi; \
    apt-get $apt_ca update \
 && apt-get $apt_ca install -y --no-install-recommends ca-certificates \
 && for list in /etc/apt/sources.list /etc/apt/sources.list.d/*; do \
        [ -f "$list" ] && sed -i 's|http://|https://|g' "$list"; \
    done; \
    apt-get $apt_ca update \
 && apt-get $apt_ca install -y --no-install-recommends git docker.io iptables procps python3 python3-pip python3-venv \
 && rm -rf /var/lib/apt/lists/* \
 && npm install --global "opencode-ai@${OPENCODE_VERSION}" \
 && npm cache clean --force \
 && opencode --version \
 && python3 --version \
 && python3 -m pip --version

# Where the runner puts things: the clones, OpenCode's managed config (a repo's own config cannot
# override it) and the skill packs.
RUN mkdir -p /work /etc/opencode /opt/antilog/skills
WORKDIR /work

ENV OPENCODE_DISABLE_MODELS_FETCH=1

# The runner starts everything else with `docker exec`.
CMD ["sleep", "infinity"]
