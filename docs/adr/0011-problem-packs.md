# ADR 0011: Problem packs and their threat model

## Context
v1.0 lets people share problems they wrote: a teacher's set for a class, a team's interview practice, a community
collection. A shared file is untrusted input that reaches the file system, the content database, the simulator and
the webview's Markdown renderer. KataBox already ships packs (its ADR 0038, with a hardened extractor and a hostile
test suite); KataBox Arch ports that design and tightens it, because an Arch problem is only text and designs: no code,
no test data.

## Decision
- **Format.** A `.kbpack` is a zip of `manifest.toml` plus `problems/<id>/` folders in the ordinary problem layout
  (`problem.toml`, the six Markdown files, `reference.kbarch`, `naive.kbarch`). The manifest has `format =
  "katabox-arch-pack/v1"`, `id` (`[a-z][a-z0-9]{2,19}`), `name`, `author`, `version` (`MAJOR.MINOR.PATCH[-pre]`),
  `licence` (SPDX characters), `min_app`, and `files`: every other file with its SHA-256. `ka-pack pack` writes one
  deterministically (sorted entries, fixed timestamps) and then opens it as the app would.
- **Where checks live.** `ka-pack` (`kbpack`, `kbpack::archive`) opens and checks a pack and builds its database;
  `ka-app` (`packs`) owns versions, staging, install and removal, and receives the checks as a `PackOpener` the app
  shell plugs in at startup (`ka-pack` depends on `ka-app`, not the other way round).
- **Storage.** An installed pack lives in `<data>/packs/<id>/` with its own `content.db` (problems only, stored as
  `<pack>--<id>`). `ka-app` opens every installed pack's database next to the bundled one, so a pack is live as soon
  as it is applied. `user.db` v6 records installs in `packs` (id, name, author, version, licence, SHA-256, installed
  and removed times); uninstalling deletes `packs/<id>` and keeps the row, so the history of its problems keeps a name.
- **The UI.** "Import pack…" (Problems and Settings): Rust opens the file dialog, then the preview shows the name,
  author, version, licence, problems, SHA-256 and the sentence "Unsigned pack, from <author> (as stated in the pack)".
  Nothing is installed until the user confirms. Problems from packs carry a badge and a source filter, and stay out of
  Home's "Recommended next" unless `packs.recommend` is on. Settings lists packs with Uninstall.

### Threat model
Each row has tests: T1–T9 in `crates/ka-pack/tests/pack_hostile.rs`, T10–T11 in `crates/ka-app/src/packs.rs`.

| | Threat | Defence |
|---|---|---|
| T1 | Zip slip, confusing names | Names are UTF-8 `[A-Za-z0-9._-]` segments: no `..`, `.` or leading dot, no absolute paths, drive letters, backslashes or alternate streams, no reserved device names, at most 8 levels and 180 characters, no two names equal ignoring case; files created with `create_new` |
| T2 | Links and special files | Regular files and folders only (Unix mode checked), no encrypted entries, stored or deflate only |
| T3 | Bombs and oversize | The archive at most 16 MiB and 1,000 entries; sizes counted on the real decompressed bytes (a capped reader): 1 MiB per `.kbarch`, 256 KiB per other file, 32 MiB in all; at most 200:1 above 64 KiB |
| T4 | Unexpected file types | Extensions `toml`, `md`, `kbarch` only |
| T5 | Tampering | Every file listed in the manifest with a matching SHA-256; unlisted, missing or out-of-place files refused |
| T6 | Spoofed ids and metadata | The pack id isn't `core`, `flaw` or a built-in problem id; problems are stored as `<pack>--<id>`; name and author 1–80 plain characters; licence in SPDX characters |
| T7 | Markup in text | Markdown with `<script`, `<iframe`, `<object`, `<embed`, `<style`, `javascript:`, `vbscript:`, `data:` or an `on…=` handler is refused; pack text also renders with raw HTML skipped (as all content), links as plain text and images as their alt text |
| T8 | Invalid content or designs | The same loading (the strict `.kbarch` parser) and structural validation as built-in content, then the engine: the reference must be Accepted on every test and the naive design must fail a hidden one |
| T9 | Simulation DoS | At most 50 problems, 10 tests, 12 request types, 120 s per test, 150 cards per design and 1,000,000 req/s; the checks run on a blocking thread |
| T10 | Downgrade, same-version tamper | An installed pack is replaced only by a newer version; the same version with the same SHA-256 is a no-op, with a different one it is refused; older versions are refused; a removed pack may come back |
| T11 | Path writes | Extraction goes into a fresh `packs/.staging-*` folder the app names; Apply renames it to `packs/<id>`, Cancel and any failure delete it, and leftovers are deleted at startup; every path is built from the validated pack id, never from text in the pack |

## Consequences
- Packs are unsigned. The preview says so and shows the SHA-256, so a teacher can publish the hash next to the file.
  Signing could come later without changing the format (a detached signature beside the manifest).
- The checks judge every design when a pack is opened, so a large pack takes a few seconds to preview.
- Built-in problems may use longer tests than packs (Set 2's Webhook Delivery runs 150 s); `ka-pack pack` reports
  the cap when an author packs such a problem.
- An app update that changes `content.db`'s schema must rebuild each installed pack's database from its kept files
  (`packs/<id>/problems/`), as KataBox does; not needed yet (schema 2 since packs arrived).
