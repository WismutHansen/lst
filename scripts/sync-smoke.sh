#!/usr/bin/env bash
# smoke-test sync pipeline across server and daemon

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVER_PORT="${LST_SMOKE_PORT:-8765}"

command -v python3 >/dev/null || {
  echo "python3 is required for the sync smoke test" >&2
  exit 1
}
command -v curl >/dev/null || {
  echo "curl is required for the sync smoke test" >&2
  exit 1
}

cargo build --quiet --bins

WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/lst-sync-smoke.XXXXXX")"
KEEP_WORKDIR="${LST_SMOKE_KEEP:-0}"

cleanup() {
  if [[ -n "${SERVER_PID:-}" ]]; then
    kill "$SERVER_PID" 2>/dev/null || true
  fi
  if [[ -n "${SYNCD_PID:-}" ]]; then
    kill "$SYNCD_PID" 2>/dev/null || true
  fi
  if [[ "$KEEP_WORKDIR" != "1" ]]; then
    rm -rf "$WORKDIR"
  else
    echo "Preserving workdir: $WORKDIR" >&2
  fi
}

trap cleanup EXIT

CONFIG_DIR="$WORKDIR/config"
STATE_DIR="$WORKDIR/state"
CONTENT_DIR="$WORKDIR/content"
CRDT_DIR="$WORKDIR/crdt"
SERVER_DATA="$WORKDIR/server-data"
KEY_FILE="$WORKDIR/master.key"
LOCAL_SYNC_DB="$WORKDIR/local-sync.db"

mkdir -p "$CONFIG_DIR" "$STATE_DIR" "$CONTENT_DIR/lists" "$CONTENT_DIR/notes" "$CRDT_DIR" "$SERVER_DATA"

cat >"$CONFIG_DIR/config.toml" <<EOF
[paths]
content_dir = "${CONTENT_DIR}"

[storage]
crdt_dir = "${CRDT_DIR}"

[sync]
server_url = "ws://127.0.0.1:${SERVER_PORT}/api/sync"
encryption_key_ref = "${KEY_FILE}"
interval_seconds = 2
max_file_size = 10485760
exclude_patterns = []

[server]
host = "127.0.0.1"
port = ${SERVER_PORT}
data_dir = "${SERVER_DATA}"
tokens_db = "tokens.db"
content_db = "content.db"
sync_db = "sync.db"

[database]
data_dir = "${SERVER_DATA}"
tokens_db = "tokens.db"
content_db = "content.db"
sync_db = "sync.db"
EOF

python3 - <<'PY' >"${WORKDIR}/jwt.txt"
import base64, json, time, hmac, hashlib

def b64(data):
    raw = json.dumps(data, separators=(",", ":"), sort_keys=True).encode()
    return base64.urlsafe_b64encode(raw).decode().rstrip("=")

header = {"alg": "HS256", "typ": "JWT"}
payload = {
    "sub": "sync-smoke@example.com",
    "exp": int(time.time()) + 3600,
}
message = ".".join([b64(header), b64(payload)])
signature = hmac.new(
    b"lst-jwt-demo-secret-goes-here",
    message.encode(),
    hashlib.sha256,
).digest()
token = message + "." + base64.urlsafe_b64encode(signature).decode().rstrip("=")
print(token)
PY

JWT_TOKEN="$(cat "${WORKDIR}/jwt.txt")"

cat >"$STATE_DIR/state.toml" <<EOF
[auth]
email = "sync-smoke@example.com"
auth_token = "dummy-auth-token"
jwt_token = "${JWT_TOKEN}"
jwt_expires_at = "2099-01-01T00:00:00Z"

[device]
device_id = "sync-smoke-device"

[sync]
database_path = "${LOCAL_SYNC_DB}"
EOF

head -c 32 /dev/urandom >"$KEY_FILE"

export LST_CONFIG="${CONFIG_DIR}/config.toml"
export LST_STATE="${STATE_DIR}/state.toml"

cargo run --quiet --bin lst-server -- --config "${LST_CONFIG}" serve >"${WORKDIR}/server.log" 2>&1 &
SERVER_PID=$!

for _ in {1..30}; do
  if curl -fsS "http://127.0.0.1:${SERVER_PORT}/api/health" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done

if ! curl -fsS "http://127.0.0.1:${SERVER_PORT}/api/health" >/dev/null 2>&1; then
  echo "Server failed to start. Logs:" >&2
  cat "${WORKDIR}/server.log" >&2 || true
  exit 1
fi

cargo run --quiet --bin lst-syncd -- --config "${LST_CONFIG}" --foreground --verbose >"${WORKDIR}/syncd.log" 2>&1 &
SYNCD_PID=$!

sleep 2

cat >"${CONTENT_DIR}/lists/smoke.md" <<'EOF'
## Sync Smoke Checklist
- [ ] first item
- [ ] second item
EOF

cat >"${CONTENT_DIR}/notes/smoke-note.md" <<'EOF'
---
title: "Sync Smoke Note"
created: 2025-01-01T00:00:00Z
---

This is a smoke-test note to validate sync.
EOF

sleep 35

python3 - <<PY
import os, sqlite3, sys, time, json

db_path = os.path.join("${SERVER_DATA}", "sync.db")
deadline = time.time() + 10
while time.time() < deadline:
    if os.path.exists(db_path):
        break
    time.sleep(0.5)
else:
    print(json.dumps({"ok": False, "reason": "sync.db not created"}))
    sys.exit(0)

conn = sqlite3.connect(db_path)
try:
    cur = conn.execute("SELECT doc_id, length(encrypted_snapshot) FROM documents")
    docs = cur.fetchall()
finally:
    conn.close()

if len(docs) < 2:
    print(json.dumps({"ok": False, "reason": "expected at least 2 documents", "documents": docs}))
    sys.exit(1)

print(json.dumps({"ok": True, "documents": docs}))
PY

echo
echo "Server log: ${WORKDIR}/server.log"
echo "Syncd log: ${WORKDIR}/syncd.log"
if [[ "$KEEP_WORKDIR" == "1" ]]; then
  echo "Workdir preserved at: ${WORKDIR}"
else
  echo "Set LST_SMOKE_KEEP=1 to keep the scratch workspace (current: ${WORKDIR})"
fi
