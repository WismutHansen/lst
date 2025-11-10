# 2025-02-19 Remove vendored glib patch

- Dropped the `[patch.crates-io]` override that pointed `glib` at `third_party/glib-0.18.5-patched`; we now consume the upstream crate directly, so there is no extra vendored source to maintain.
- Verified the workspace builds successfully with `cargo check` after removing the override.
- Follow-up work: when GTK/tauri moves to gtk-rs ≥0.20, bump the dependencies so we get the upstream `VariantStrIter` fix directly instead of relying on the old 0.18.x line.
