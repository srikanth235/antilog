# The image every session's container starts from (`antilog runner --image`). It holds the harness
# (OpenCode, one pinned version), git and Docker for the agent. It holds no infrastructure tool:
# no OpenTofu, Terraform or cloud CLI (docs/V0.md). An environment's setup script adds what a
# project needs on top: the runner runs it in each fresh container after cloning the repos.
#
#   docker build -t antilog-session:latest session-image
#   (behind a TLS-inspecting proxy add: --secret id=ca,src=/path/to/ca.crt)
FROM node:22-bookworm-slim

# The pinned OpenCode: the version `.claude/skills/session/runner.md` was verified against.
ARG OPENCODE_VERSION=1.19.6

# `docker.io` is the client and the daemon. The runner starts the daemon in the container only
# when the host runs it under sysbox, so the agent has Docker without `--privileged`.
# Behind a TLS-inspecting proxy, pass its CA as a build secret (`--secret id=ca,src=ca.crt`):
# npm trusts it for this step only and it never lands in the image.
RUN --mount=type=secret,id=ca,required=false \
    apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates git docker.io iptables procps \
 && rm -rf /var/lib/apt/lists/* \
 && if [ -s /run/secrets/ca ]; then export NODE_EXTRA_CA_CERTS=/run/secrets/ca; fi \
 && npm install --global "opencode-ai@${OPENCODE_VERSION}" \
 && npm cache clean --force \
 && opencode --version

# Where the runner puts things: the clones, OpenCode's managed config (a repo's own config cannot
# override it) and the skill packs.
RUN mkdir -p /work /etc/opencode /opt/antilog/skills
WORKDIR /work

ENV OPENCODE_DISABLE_MODELS_FETCH=1

# The runner starts everything else with `docker exec`.
CMD ["sleep", "infinity"]
