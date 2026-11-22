# Proposal: links between KataBox and KataBox Arch

Status: proposal (M6.7). Nothing here is built yet; it needs work in both apps, and the KataBox part is a KataBox
task.

## Today
KataBox Arch shows a "Related in KataBox" box on Guide entries, patterns and problem briefs: the titles of the
KataBox Learn topics and problems that drill the algorithm behind the page (an LRU cache for the Cache entry, a trie
for Typeahead, Dijkstra for Ride Sharing). The links live in `content/crosslinks.toml`; the KataBox ids and titles
they point at are a read-only snapshot in `content/katabox-ids.toml` (with the KataBox commit it was taken from),
and `ka-pack validate` checks every link resolves. The titles are not clickable: the apps are separate, offline
programs and neither can open the other.

## Option A: a `katabox://` URL scheme (recommended)
- **KataBox** registers a URL scheme at install time (Tauri's deep-link plugin; Windows registry, Linux `.desktop`
  `MimeType`). `katabox://learn/<subject>/<topic>` and `katabox://problem/<id>` open that page, starting the app if
  needed. Unknown or malformed paths open Home; the path is matched against an allow-list of route shapes, never
  passed to a shell.
- **KataBox Arch** turns each title into a button that asks Rust to open the URL with the system opener (only the two
  shapes above, built from snapshot ids, never from user text). If KataBox isn't installed, the OS reports that no
  app handles the scheme; Arch says "Install KataBox to practise this" instead.
- **Security and privacy:** no network; the URL carries only a public content id. Both apps keep CSP and
  capabilities as they are; the opening happens in Rust. This needs the maintainers' sign-off, because registering a
  scheme is an installer change.
- **The reverse direction:** KataBox's Learn pages could show "See it at scale in KataBox Arch" with
  `katabox-arch://guide/<kind>` or `katabox-arch://problem/<id>` the same way, from a snapshot of Arch's ids kept in
  KataBox's content (validated there in the same way).

## Option B: a shared file
Both apps write a small `installed.json` (app, version, content ids) into a shared folder under the user's data
directory, and show cross-links only for the other app when it is present. This tells each app whether the other is
installed and which ids exist (no stale snapshot), but it still can't open the other app, so it only complements
Option A.

## Keeping the snapshot honest
- Refresh `katabox-ids.toml` from a KataBox checkout (titles in English and Spanish, plus the commit) when KataBox
  renames or removes content; `ka-pack validate` fails if a link no longer resolves.
- A KataBox CI check could fail when a renamed id is still listed in Arch's snapshot, once Option A exists.
