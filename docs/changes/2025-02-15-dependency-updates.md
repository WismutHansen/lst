# 2025-02-15 Dependency Updates

- Updated the workspace lockfile to pull in `slab` v0.4.11, addressing CVE-2025-55159.
- Patched `glib` v0.18.5 locally to fix the unsound `VariantStrIter` pointer handling reported upstream and wired the workspace through `[patch.crates-io]`.
- Added the patched crate under `third_party/` so builds consume the hardened source.
- Ran `cargo check` to verify the workspace still compiles without regressions.
