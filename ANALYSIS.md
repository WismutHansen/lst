**Executive Summary**
- **Problem:** Initial sync works, but concurrent edits on different clients diverge and do not converge. Users see mismatched content that won’t sync over.
- **Primary causes:**
  - Unstable `doc_id` derivation based on absolute file paths per device.
  - Server only broadcasts live changes; it does not replay stored changes or maintain a canonical CRDT state.
  - Automerge text usage is inconsistent, leading to lossy reads/writes for notes.
  - Locally written files from remote changes are re-processed as local edits (feedback loop) because the watcher ignore list is not used.
  - JWT refresh endpoint is referenced by clients but not implemented on the server.
- **Approach:** Keep end-to-end encryption and a “mailbox server” model. Stabilize IDs, fix CRDT usage, implement reliable change replay + compaction, and add loop protection in the watcher. Prioritize a minimal set of concrete changes to restore correct, convergent sync.

**Current Architecture**
- **Data model:** Markdown files under `content/` (`lists/` and `notes/`).
- **Client-side CRDT:** `lst-syncd` uses Automerge. One CRDT doc per markdown file. Local SQLite (`documents` table) stores: `doc_id`, `file_path`, `doc_type`, `last_sync_hash`, `automerge_state`, owner and ACLs.
- **Transport:** WebSocket (`/api/sync`). Messages in `lst-proto`:
  - Client → Server: `RequestDocumentList`, `RequestSnapshot`, `PushChanges`, `PushSnapshot`, `Authenticate`.
  - Server → Client: `Authenticated`, `DocumentList`, `Snapshot`, `NewChanges`, `RequestCompaction` (unused).
- **Server:**
  - `documents` table: snapshot blob + encrypted filename; used for `DocumentList` and `Snapshot`.
  - `document_changes` table: append-only list of encrypted changes; on `PushChanges`, server stores and broadcasts changes to currently connected devices only.
- **Security:** E2E encryption with XChaCha20-Poly1305; server stores opaque changes/snapshots. Key derived on clients from email + password + auth token; stored on disk (base64).

**Root Causes Of Broken Sync**
- **Unstable document identity (critical):**
  - `lst-syncd` generates `doc_id = UUIDv5(NAMESPACE_OID, absolute_path_bytes)` during file events.
  - Different devices have different absolute content roots; the same logical document gets different `doc_id`s across devices.
  - On first join, devices ingest server snapshots under the server’s `doc_id`. Later local edits trigger a new `doc_id` derived from the device’s absolute path ⇒ duplicate doc records and no merging across devices.
  - Evidence: `handle_file_event` derives `doc_id` from `path.to_string_lossy()`, and `insert_new_document_from_snapshot_with_filename` persists the server’s `doc_id` mapped to a full path; subsequent events don’t look up by path first.

- **No change replay for offline devices (critical):**
  - Server stores changes in `document_changes` but never replays them when a device connects later.
  - Only live broadcasts are sent; if a device is offline, it misses changes and won’t catch up (snapshots are not updated automatically).
  - `DocumentList` is built solely from `documents` (snapshots). Changes alone don’t appear if no snapshot exists.

- **Server snapshot not maintained (design gap):**
  - With E2E encryption the server cannot merge changes into a canonical doc; therefore the snapshot only updates when clients push a snapshot (rare except initial seed). New changes aren’t reflected in snapshots.

- **Automerge text misuse (contributor to divergence/empties):**
  - Notes: `update_note_doc` uses `tx.put(ROOT, "content", "")` then `tx.update_text(&ROOT, content)`. This does not ensure a `Text` object at `content`.
  - Reads later attempt `doc.text(content_id)` or scalar string. Inconsistent shapes cause empty or lossy content extraction.

- **Feedback loops on file writes:**
  - `apply_remote_changes` writes to the file but does not mark the file as "recently synced"; the watcher then re-ingests that write as a local change and re-uploads.
  - A `recently_synced_files` set exists but is never populated.

- **JWT refresh flow broken:**
  - Clients call `POST /api/auth/refresh`, but the server does not implement this endpoint. Sync aborts on expiry or near-expiry.

- **Minor issues and risks:**
  - Hard-coded `Host` header in WS request: `192.168.1.25:5673` (breaks non-local servers).
  - `DocumentList` ignores docs that only have changes recorded (no snapshot yet).
  - `detect_doc_type` relies on path heuristics despite DB carrying `doc_type`.
  - Filename/path validation in `LocalDb` tries to “fix” paths; better is to store normalized relative paths and content-root separately.
  - Lack of backpressure and dedupe in change flooding scenarios; no debounce of file watcher events.

**Minimal Changes To Restore Correctness (Low Risk, High Impact)**
- **Stable doc identifiers:**
  - Define `relative_path = file_path.strip_prefix(content_dir)` and use a normalized form (forward slashes, lowercase dirs) for identity.
  - Compute `doc_id = UUIDv5(NAMESPACE_URL, b"lst://" + relative_path_bytes)`. Do NOT use absolute paths.
  - On file events:
    - First: look up by `file_path` in `documents`; if present, use stored `doc_id`.
    - Else: compute `relative_path` and derive `doc_id` once, then persist mapping; never recompute from absolute path.
  - On snapshot ingest: persist server `doc_id` mapped to the locally reconstructed full path; subsequent events must reuse this `doc_id`.

- **Automerge text model fix (notes):**
  - Create a `Text` object under `content`: `put_object(ROOT, "content", ObjType::Text)` on first creation; then `splice_text` or `update_text(content_id, ...)` consistently.
  - For reads, always prefer `doc.text(content_id)` for notes; avoid storing scalars for `content`.

- **Prevent feedback loops:**
  - After `apply_remote_changes` writes a file, insert that path into `recently_synced_files`. Skip and remove from set on the next file event for that path.
  - Optionally add a short TTL or debounce window.

- **JWT refresh made real:**
  - Implement `POST /api/auth/refresh` on the server to mint a new JWT using the stored token/password-hash.
  - Or change clients to call `/api/auth/verify` again using the stored token (simplest), and remove refresh endpoint usage.

- **Server DocumentList completeness:**
  - Include docs that have only changes (no snapshot) by `UNION` with `SELECT DISTINCT doc_id FROM document_changes` and default blank filename/updated_at.

These changes alone will:
- Ensure all devices reference the same `doc_id` per document.
- Allow proper CRDT merges for simultaneously edited docs when devices are online together.
- Avoid re-upload loops after applying remote changes.
- Keep authentication working beyond 1 hour.

**Reliable Sync For Offline Devices (Next Step, Still Simple)**
- Keep the server as an E2E-encrypted mailbox; add deterministic, resumable change replay:
  - Protocol additions:
    - `ClientMessage::RequestChanges { doc_id: Uuid, since_change_id: Option<i64> }`
    - `ServerMessage::Changes { doc_id: Uuid, from_change_id: i64, changes: Vec<Vec<u8>> }`
    - `ClientMessage::AckChanges { doc_id: Uuid, up_to_change_id: i64 }` (optional if client keeps last seen locally)
  - Client responsibilities:
    - Track `last_seen_change_id` per `(doc_id)` in local DB.
    - On connect, after `DocumentList`, request changes for each known doc with `since_change_id`.
    - Apply changes, update `last_seen_change_id`, then write files and DB snapshot.
  - Server responsibilities:
    - Query `document_changes WHERE doc_id = ? AND change_id > since_change_id ORDER BY change_id`.
    - Send batched `Changes` messages; optionally chunk by size.
    - When change-log length per doc exceeds N, send `RequestCompaction { doc_id }` to ask a client to push a fresh snapshot.

This preserves E2E encryption (server never decrypts) while guaranteeing convergence for offline devices.

**Medium-Term Improvements**
- **Periodic client snapshot push:** After every M applied changes or K minutes, client sends `PushSnapshot` to refresh the server’s snapshot. Helps newcomers and reduces replay volume.
- **Debounce file watcher:** Coalesce rapid successive events (e.g., 200–500ms window) to avoid excessive CRDT commits.
- **Backoff and retry:** Exponential backoff for WS reconnects and HTTP calls; jittered intervals.
- **Remove hard-coded headers:** Let tungstenite set `Host` and `Sec-WebSocket-Key`; derive host from configured URL only.
- **Stronger type signaling:** Store and rely on `doc_type` in local DB; only fall back to path heuristics during first import.
- **Observability:** Adopt `tracing` with structured fields (doc_id, device_id, change_id) across client/server for easier debugging.
- **Tests:** Add an integration test simulating two devices:
  - Start server, create note on device A, connect B, ensure convergence.
  - Apply concurrent edits A/B while B is offline, ensure B catches up via change replay upon reconnect.

**Risks & Trade-offs**
- **Mailbox model limitations:** Server cannot perform server-side merges or validation. Clients must be robust to duplicates and apply idempotent merges.
- **Change-log growth:** Without compaction, `document_changes` can grow indefinitely; mitigate via client-triggered snapshots on threshold or server `RequestCompaction`.
- **Key management:** All a user’s devices must share the same derived key. Document the login + key derivation flow clearly and keep the on-disk key secure.

**Concrete Implementation Notes (File-level)**
- `crates/lst-syncd/src/sync.rs`:
  - Derive and use stable `doc_id` from normalized relative path; first try DB lookup by `file_path` to reuse `doc_id`.
  - Fix notes text:
    - On create: `let content_id = tx.put_object(ROOT, "content", ObjType::Text)?; tx.update_text(&content_id, content)?;`
    - On update: fetch `content_id`; ensure it’s `Text`; then update.
  - After `apply_remote_changes` writes a file: insert `file_path` into `recently_synced_files`.
  - Remove hard-coded `Host` header; use URL host.
  - If JWT refresh remains, change to call `/api/auth/verify` with stored token instead of `/api/auth/refresh` unless server adds refresh.

- `crates/lst-syncd/src/database.rs`:
  - Store normalized `relative_path` alongside `file_path`, or compute and store `relative_path` then build `file_path` via `content_dir` on load. Use it to derive stable `doc_id` if missing.
  - Avoid over-aggressive path “fixing”; prefer explicit normalization.

- `crates/lst-server/src/main.rs` and `sync_db.rs`:
  - Add `/api/auth/refresh` or endorse `verify` re-use for refresh.
  - Extend WS handler and DB with `RequestChanges`/`Changes`/`AckChanges`.
  - Update `DocumentList` to include docs that only appear in `document_changes`.
  - Implement `RequestCompaction` threshold logic.

- `crates/lst-proto/src/lib.rs`:
  - Add the new messages for change replay and acknowledgements.

**Suggested Rollout Plan**
- Phase 1 (stabilize):
  - Stable `doc_id` by relative path; CRDT text fix; feedback-loop guard; remove hard-coded WS headers; fix JWT refresh flow; extend `DocumentList`.
  - Manual verification: two devices online concurrently; confirm merges converge.
- Phase 2 (reliability):
  - Add change replay protocol and server support; client maintains per-doc `last_seen_change_id`.
  - Add periodic snapshot push and server compaction request handling.
- Phase 3 (polish):
  - Debounce watcher; backoff strategy; tracing; integration tests.

**Checklist Of Immediate Code Changes**
- `doc_id` derivation: relative path based, with DB lookup by `file_path` first.
- Automerge notes: always `ObjType::Text` under key `content`; consistent update/read.
- Watcher loop protection: populate and honor `recently_synced_files`.
- WS client: remove hard-coded headers; rely on configured URL.
- Auth: either implement `/api/auth/refresh` or switch client to re-`/api/auth/verify` using stored token.
- `DocumentList`: union with docs that only have changes.

With these changes, sync will converge for both live and (after Phase 2) offline devices, while keeping the system simple and end-to-end encrypted.
