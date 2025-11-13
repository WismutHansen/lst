# 2025-11-10 Mobile sync encryption key path alignment

- Updated the mobile sync configuration builder to point at the same master-key location that secure login writes to, preventing fresh installs from referencing an unused `sync.key` file.
- Added a fallback loader in the mobile sync runtime so devices with older configs can still locate the derived key and re-establish sync without re-authenticating.
- Mobile users no longer encounter “Authentication required: no encryption key available” immediately after login.
