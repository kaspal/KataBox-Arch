# Packaging

## Versions

The app version lives in three files and must match: `Cargo.toml` (`[workspace.package]`),
`apps/desktop/package.json` and `apps/desktop/src-tauri/tauri.conf.json`. `pnpm versions:check`
checks them, Cargo.lock, and the MSI version.

Windows Installer compares only three numeric fields, so `bundle.windows.wix.version` is derived
from the app version by `msiVersion` in `scripts/check-versions.mjs` (KataBox's rule, unchanged):

| App version | MSI version |
| --- | --- |
| `0.M.p` (before 1.0: plain versions only, minor ≤ 98) | `0.M.p` |
| `1.0.0-beta.N` | `0.99.N` |
| `1.0.0-rc.N` | `0.99.(100+N)` |
| `M.m.p` | `M.m.p` |

The 0.x line is itself the pre-release line, so `0.x.y-beta.N` is refused.

## Signing and updates

Off until the maintainer turns them on. The `updater` cargo feature and `tauri.updater.conf.json`
exist with a placeholder key; never generate, commit or ask for a signing or updater key.

## Third-party notices

`pnpm notices` writes `apps/desktop/src-tauri/resources/THIRD_PARTY_NOTICES.txt` (Rust crates,
npm packages, `docs/third-party-notices.md`). Release builds refuse the development placeholder.
