# 2025-11-09 Sync Path Normalization

- Normalized all sync-daemon document records to store canonical relative paths, eliminating duplicate doc IDs when multiple machines write to the same list files.
- Added automatic migrations (and a `lst-syncd --migrate-only` flag) that rewrite existing SQLite rows before the daemon starts, plus enhanced lookups that fall back to legacy absolute paths during the transition.
- Updated snapshot handling so uploads/downloads now rely on the relative path directly, keeping filenames stable across platforms.
- Fixed a panic in `update_list_doc` by inserting new list items sequentially instead of using the original line indices, so blank lines no longer trigger “index out of bounds” crashes during sync.
- Ran `cargo check` for the workspace to ensure the refactor builds cleanly.
