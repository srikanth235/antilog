#!/usr/bin/env bash
# One local end-to-end run of Antilog: Postgres, the server, the proxy and a runner on this
# machine, one session in a real container, OpenCode talking to a model through the proxy.
#
#   ZEN_KEY=... scripts/local-run.sh                  # OpenCode Zen, a free model
#   MODEL=opencode/big-pickle ZEN_KEY=... scripts/local-run.sh
#   SERVE_ONLY=1 MODEL_KEY=x scripts/local-run.sh      # only the stack, to drive from the web app
#   MODEL=opencode-go/glm-5.1 MODEL_KEY=oc_sk_... scripts/local-run.sh   # OpenCode Go
# The provider is the part of MODEL before the slash (anthropic, openai, opencode, opencode-go);
# its key is MODEL_KEY, or ZEN_KEY as before.
#
# Needs: Docker (running), a Rust toolchain, curl, python3, openssl. About 3 GB of disk for the
# session image. Everything it starts is stopped on exit; its files live in .local-run/.
# Behind a TLS-inspecting proxy set EXTRA_CA=/path/to/ca.crt so the image build trusts it.
set -euo pipefail

cd "$(dirname "$0")/.."
: "${MODEL_KEY:=${ZEN_KEY:-}}"
: "${MODEL_KEY:?set MODEL_KEY (or ZEN_KEY) to the API key of the provider of MODEL}"
MODEL="${MODEL:-opencode/big-pickle}"
PROVIDER="${MODEL%%/*}"
PROMPT="${PROMPT:-List the tools you have from the antilog MCP server by name, then reply with the single word ready.}"
WAIT_SECS="${WAIT_SECS:-300}"
PROFILE="${PROFILE:-release}"   # or debug, to reuse a debug build
BIN="target/$PROFILE"
RUN=.local-run
PG_PORT="${PG_PORT:-55432}"
SERVER="http://127.0.0.1:8080"
umask 077

step() { printf '\n== %s\n' "$*"; }
die() { printf 'local-run: %s\n' "$*" >&2; exit 1; }
for tool in docker cargo curl python3 openssl; do
  command -v "$tool" >/dev/null || die "$tool is not installed"
done
docker info >/dev/null 2>&1 || die "Docker is not running"

pids=()
cleanup() {
  for pid in "${pids[@]}"; do kill "$pid" 2>/dev/null || true; done
  docker rm -f antilog-local-pg >/dev/null 2>&1 || true
  for c in $(docker ps -aq --filter label=antilog.session); do docker rm -f -v "$c" >/dev/null 2>&1 || true; done
  for n in $(docker network ls -q --filter label=antilog.session); do docker network rm "$n" >/dev/null 2>&1 || true; done
}
trap cleanup EXIT

step "Building the server, the proxy and the runner"
if [ "$PROFILE" = release ]; then
  cargo build --release -q -p antilog-server -p antilog-proxy -p antilog
else
  cargo build -q -p antilog-server -p antilog-proxy -p antilog
fi

step "Building the session image"
secret=()
[ -n "${EXTRA_CA:-}" ] && secret=(--secret "id=ca,src=$EXTRA_CA")
if [ -z "${REBUILD:-}" ] && docker image inspect antilog-session:latest >/dev/null 2>&1; then
  echo "using the existing image (REBUILD=1 rebuilds it)"
else
  docker build -q "${secret[@]}" -t antilog-session:latest session-image >/dev/null
fi

step "Starting Postgres on port $PG_PORT"
docker rm -f antilog-local-pg >/dev/null 2>&1 || true
docker run -d --name antilog-local-pg -e POSTGRES_PASSWORD=pw -p "127.0.0.1:$PG_PORT:5432" "${PG_IMAGE:-postgres:18}" >/dev/null
for _ in $(seq 1 60); do docker exec antilog-local-pg pg_isready -U postgres >/dev/null 2>&1 && break; sleep 1; done
export DATABASE_URL="postgres://postgres:pw@127.0.0.1:$PG_PORT/postgres"

# Fresh secrets for this run only.
export ANTILOG_SECRET_MODEL_KEY_MASTER="$(openssl rand -hex 32)"
export ANTILOG_SECRET_MASTER_KEY="$(openssl rand -hex 32)"
export ANTILOG_RUNNER_KEY="$(openssl rand -hex 24)"

step "Starting the server and the proxy (logs in $RUN/)"
# The session's container reaches only the proxy: model calls go to /<provider>/v1 and MCP calls
# to /mcp, which the proxy passes on to the server. The host names are never resolved inside it.
ANTILOG_LISTEN=0.0.0.0:8080 ANTILOG_FAKE_SIGN_IN=1 ANTILOG_OPEN_ORGANIZATIONS=1 \
  ANTILOG_INSECURE_COOKIES=1 ANTILOG_MODEL="$MODEL" \
  ANTILOG_PROXY_URL=http://antilog-proxy:8081 ANTILOG_PUBLIC_URL=http://antilog-proxy:8081 \
  "$BIN"/antilog-server >"$RUN/server.log" 2>&1 &
pids+=($!)
ANTILOG_PROXY_LISTEN=0.0.0.0:8081 ANTILOG_PROXY_SERVER_URL=http://127.0.0.1:8080 ANTILOG_PROXY_PLATFORM_HOSTS=antilog-server,antilog-proxy,github.com \
  "$BIN"/antilog-proxy >"$RUN/proxy.log" 2>&1 &
pids+=($!)
for _ in $(seq 1 60); do curl -fsS "$SERVER/auth/providers" >/dev/null 2>&1 && break; sleep 1; done
curl -fsS "$SERVER/auth/providers" >/dev/null || die "the server did not start; see $RUN/server.log"

api() { # method path [json]
  local args=(-fsS -X "$1" "$SERVER$2" -H "authorization: Bearer $TOKEN")
  [ $# -ge 3 ] && args+=(-H 'content-type: application/json' -d "$3")
  curl "${args[@]}"
}
field() { python3 -c "import json,sys; d=json.load(sys.stdin); print(eval('d'+sys.argv[1]))" "$1"; }

if [ -n "${SERVE_ONLY:-}" ]; then
  step "Serving for the web app (Ctrl-C stops everything)"
  "$BIN"/antilog runner --server "$SERVER" --name local --proxy 127.0.0.1:8081 >"$RUN/runner.log" 2>&1 &
  pids+=($!)
  echo "API on $SERVER. Run the web app: cd web && ANTILOG_API=$SERVER pnpm dev, then open"
  echo "http://localhost:5173, sign in with a test account, and add the model key under Settings."
  wait
  exit 0
fi

step "Signing up a test owner and organization"
jar="$RUN/cookies"
state=$(curl -fsS -c "$jar" "$SERVER/auth/fake/start" | field "['state']")
TOKEN=$(curl -fsS -b "$jar" -X POST "$SERVER/auth/fake/organization" -H 'content-type: application/json' \
  -d "{\"name\":\"local\",\"code\":\"fake:ada\",\"state\":\"$state\"}" | field "['session']['secret']")

step "Saving the model key (sealed) for provider $PROVIDER"
api PUT "/model-keys/$PROVIDER" "{\"key\":\"$MODEL_KEY\"}" >/dev/null

step "Starting the runner"
"$BIN"/antilog runner --server "$SERVER" --name local --proxy 127.0.0.1:8081 >"$RUN/runner.log" 2>&1 &
pids+=($!)

step "Starting a session on $MODEL with a \$5 budget"
body=$(python3 -c 'import json,sys; print(json.dumps({"agent":"opencode","prompt":sys.argv[1],"repos":[],"budgetCapCents":500}))' "$PROMPT")
session=$(api POST /sessions "$body" | field "['id']")
echo "session $session"

step "Waiting up to ${WAIT_SECS}s for the session"
seen=0
worked=
for _ in $(seq 1 $((WAIT_SECS / 5))); do
  sleep 5
  status=$(api GET "/sessions/$session" | field "['status']" 2>/dev/null || echo unknown)
  api GET "/sessions/$session/events" >"$RUN/events.json" 2>/dev/null || true
  seen=$(python3 - "$RUN/events.json" "$seen" <<'EOF'
import json, sys
try:
    events = json.load(open(sys.argv[1]))
except Exception:
    print(sys.argv[2]); sys.exit()
events = events.get("events", events) if isinstance(events, dict) else events
start = int(sys.argv[2])
for e in events[start:]:
    text = e.get("text") or e.get("summary") or json.dumps(e)[:300]
    print(f"  [{e.get('kind', '?')}] {text}", file=sys.stderr)
print(len(events))
EOF
)
  # A new session waits until a runner picks it up, so "waiting_for_you" counts only after the
  # agent has worked.
  [ "$status" = working ] && worked=1
  case "$status" in
    done|failed) break ;;
    waiting_for_you) [ -n "${worked:-}" ] && break ;;
  esac
done

step "Result"
echo "status: $status"
api GET "/sessions/$session/spend" || true
echo
echo "Logs: $RUN/server.log $RUN/proxy.log $RUN/runner.log; events: $RUN/events.json"
