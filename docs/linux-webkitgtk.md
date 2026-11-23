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
- **Animated exports:** WebM needs `MediaRecorder` and `canvas.captureStream`, which WebKitGTK may lack; the menu item
  is then disabled with a note ("This system's web view can't record video"). GIF, when it lands, won't need either.
- **PDF printing (not wired yet):** `print_pdf` returns `pdf_unsupported` on Linux, and the UI offers the print-ready
  HTML. To wire it: in `src-tauri/src/pdf.rs` (`cfg(target_os = "linux")`), call `window.with_webview`, take
  `webview.inner()` (a `webkit2gtk::WebView`), build a `webkit2gtk::PrintOperation` for it, give it a
  `gtk::PrintSettings` with `set_printer("Print to File")`, `OUTPUT_URI` set to the picked path as a `file://` URI and
  `output-file-format` `pdf`, then `print()` and answer on the `finished`/`failed` signals with the same 60 s timeout.
  This needs the Linux target to compile and a Linux host to test, so it waits for the Linux phase.
- **Data folder:** `~/.local/share/dev.katabox.arch` (`-dev` for debug builds).
- **Reduced motion:** GNOME's "Reduce animation" setting should reach `prefers-reduced-motion`.

## CI
A Linux job in GitHub Actions (when Actions is available) can run `cargo test`, `pnpm test` and
`pnpm release:build` on `ubuntu-22.04` with the packages above, and upload the three bundles as artefacts.
