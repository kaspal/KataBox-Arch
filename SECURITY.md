# Security policy

KataBox Arch is an offline desktop app. It never runs code from problems or designs: a design is data that the
built-in simulator reads. Untrusted input enters the app through the files you open (`.kbarch` designs, `.kbpack`
problem packs, Docker Compose files to import) and, once they are turned on, share links. If you find a way past the
protections below, please report it privately.

## Supported versions

| Version | Supported |
| --- | --- |
| 0.1.x (development) | Yes |

Fixes ship in a new release; there is no automatic update, so users install it themselves.

## How KataBox Arch protects your machine

### The app

- The UI talks to Rust only through typed commands; it cannot read files or start processes. File dialogs are
  opened from Rust, and the UI may only name a path that came from a dialog or the recent files list.
- The webview's Content Security Policy allows only the app's own resources and its IPC channel; the app's
  capabilities are `core:default` only.
- The settings the UI may write are an allow-list; anything else is refused.
- Content (problems, the Guide, pack text) renders as Markdown with raw HTML skipped.

### Designs (`.kbarch`)

Designs are read strictly ([ADR 0003](docs/adr/0003-kbarch-format-v1.md)): a size cap, no unknown fields, and
checked values (card count, label length, known kinds, flavours and props). Opening a design never executes anything.

### Problem packs (`.kbpack`)

Packs are zips of problems, checked before anything is installed ([ADR 0011](docs/adr/0011-problem-packs.md), each
threat with tests): safe names only (no zip slip, links or special files); size and ratio caps against zip bombs;
only `toml`, `md` and `kbarch` files; every file listed in the manifest with a matching SHA-256; ids that can't spoof
built-in problems; no script or active markup in text; the same strict parsing and judging as built-in content; caps
on tests and simulation size; no downgrades; extraction only into a staging folder the app names. Packs are unsigned:
the preview shows the SHA-256 so an author can publish it next to the file.

### Share links (planned, off in this version)

When share links are turned on, a link carries a compressed design. The web viewer will decompress it with a 256 KB
cap, parse it with the same strict `.kbarch` reader and card limit (150), and render labels as text only. Opening a
link sends nothing anywhere.

## In scope

- A crafted `.kbarch`, `.kbpack` or Compose file that crashes the app, writes outside its folders or the place the
  user chose, corrupts `user.db`, or executes anything.
- A way for content shown in the app (statements, editorials, Guide entries, pack text, card labels) to run script,
  call commands it should not, or load remote resources.
- A share link (once enabled) that does any of the above in the web viewer.
- Installer and uninstaller problems, for example cleanup that touches files outside the app's folders.

## Out of scope

- **Hidden tests are not secret.** They ship as plain data with the app, by design; there is no anti-cheat.
- A design that makes the simulator slow within the documented caps.
- Attacks that need another process already running as the same user.
- Unsigned installers (SmartScreen warnings) during development.

## Reporting a vulnerability

**Do not open a public issue, discussion or pull request for a vulnerability.** Public reports put every user at risk
before a fix exists.

Report it privately through GitHub's private vulnerability reporting: open the repository's **Security** tab and
choose **Report a vulnerability**, or go straight to <https://github.com/kaspal/KataBox-Arch/security/advisories/new>.
Only the maintainer sees the report, and the fix and the advisory are prepared there before anything is published.

Please include the app version, your Windows version, what you did (the file that triggers it helps) and what
happened. We aim to acknowledge reports within 7 days.
