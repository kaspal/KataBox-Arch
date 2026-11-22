# Making a problem pack

A problem pack (`.kbpack`) shares problems you wrote with other KataBox Arch users: a set for a class, a team's
interview practice, a community collection. The app checks every pack before installing it (ADR 0011), so a pack
that installs is one that works.

## 1. Write the problems
Each problem is an ordinary problem folder; [problem-authoring.md](problem-authoring.md) covers the format. Work in a
copy of the repository (or any folder with a `problems/` directory):

```sh
cargo run -p ka-pack -- new my-problem                       # a folder with TODO markers
cargo run -p ka-pack -- try --problems my-problem            # judge the designs while you tune them
cargo run -p ka-pack -- validate --run --problems my-problem # structure, EN/ES, and the judged rule
```

The judged rule is the same as for built-in problems: the reference design is Accepted on every test, and the naive
design fails at least one hidden test.

## 2. Describe the pack
Write a small TOML file, for example `pack.toml`:

```toml
id = "netdrills"            # 3–20 lowercase letters and digits, starting with a letter
name = "Networking drills"  # 1–80 characters
author = "Ana García"       # shown to whoever imports it, "as stated in the pack"
version = "1.0.0"           # MAJOR.MINOR.PATCH, optionally -pre.1
licence = "CC-BY-SA-4.0"    # an SPDX expression
min_app = "0.1.0"           # the oldest KataBox Arch that can open it
```

## 3. Build it
```sh
cargo run -p ka-pack -- pack pack.toml --problems my-problem,other-problem --out netdrills.kbpack
```

`ka-pack pack` writes the pack (the same input always gives the same bytes) and then opens it the way the app does,
running every check and judging every design. It prints the pack's SHA-256; publish it next to the file so people
can check they got the pack you made.

## Limits
A pack is refused if it breaks any of these (they keep imports safe and fast):

| | Limit |
|---|---|
| Files | `manifest.toml` plus `problems/<id>/` folders; only `.toml`, `.md` and `.kbarch` files |
| Names | letters, digits, `.`, `_` and `-`; no hidden files; at most 8 levels and 180 characters |
| Sizes | the pack at most 16 MB and 1,000 files; a design at most 1 MB, other files 256 KB; 32 MB unpacked |
| Problems | at most 50; each at most 10 tests, 12 request types, 120 s per test, 150 cards per design, 1,000,000 req/s |
| Text | no raw HTML tags such as `<script>` or `<iframe>`, no `on…=` handlers, no `javascript:` or `data:` URLs. Links show as plain text and images as their alt text |
| Ids | the pack id can't be `core` or a built-in problem's id; problems are stored as `<pack>--<problem>` |

## Updating a pack
Raise `version` and build again. Someone who has the pack installed can import the newer one; the app refuses an
older version, and the same version with different contents. Uninstalling a pack removes its problems from the
catalogue but keeps the user's submissions.
