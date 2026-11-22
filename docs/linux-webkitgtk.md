# Linux builds (WebKitGTK)

KataBox Arch runs on Linux through Tauri's WebKitGTK webview. The bundle config already lists `deb`, `rpm` and
`appimage`; nothing has been built or tested on Linux yet (the development machine is Windows).

## Build host
Ubuntu 22.04+ (or Debian 12, Fedora 39+) with:

```sh
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libayatana-appindicator3-dev patchelf build-essential
# Fedora: sudo dnf install webkit2gtk4.1-devel gtk3-devel librsvg2-devel patchelf
pnpm install && pnpm release:build
```

Tauri adds the runtime dependencies to the `.deb` (`libwebkit2gtk-4.1-0`, `libgtk-3-0`) and `.rpm`
(`webkit2gtk4.1`) itself. The AppImage bundles them.

## What to check on Linux
- **Fonts:** the UI ships its fonts; check the canvas text and the hand-written note font render.
- **File dialogs:** Open, Save As, Import pack and the export dialogs go through `tauri-plugin-dialog` (GTK or the XDG
  portal on Flatpak-like setups).
- **Clipboard:** "Copy as Mermaid" uses the web clipboard API; WebKitGTK allows it after a click.
- **PNG export:** draws an SVG `data:` image on a canvas; check WebKitGTK decodes it (the CSP allows `data:` images).
- **Animated exports (later):** `MediaRecorder` support varies in WebKitGTK; the plan is to disable WebM there with a
  tooltip and keep GIF.
- **Data folder:** `~/.local/share/dev.katabox.arch` (`-dev` for debug builds).
- **Reduced motion:** GNOME's "Reduce animation" setting should reach `prefers-reduced-motion`.

## CI
A Linux job in GitHub Actions (when Actions is available) can run `cargo test`, `pnpm test` and
`pnpm release:build` on `ubuntu-22.04` with the packages above, and upload the three bundles as artefacts.
