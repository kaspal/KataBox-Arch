# Third-party notices

KataBox Arch is MIT OR Apache-2.0. Release packages must ship each bundled component's
license text (`LICENSE`/`COPYING` files from the component, or the npm package's license
file). Rust crates and npm packages linked into the app are checked against the
allow-lists in `deny.toml` and `scripts/check-npm-licenses.mjs`; their texts go into the
bundle's generated notices (`pnpm notices`).

## Fonts

| Component | License | Notes |
| --- | --- | --- |
| Cormorant Garamond (`@fontsource-variable/cormorant-garamond`) | OFL-1.1 | Display face |
| Geist, Geist Mono (`@fontsource-variable/geist`, `@fontsource-variable/geist-mono`) | OFL-1.1 | UI and numbers |
| Kalam (`@fontsource/kalam`) | OFL-1.1 | Sticky notes and annotations |

The fonts are bundled with the app; nothing loads from the network.
