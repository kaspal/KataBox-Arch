# ADR 0003: The `.kbarch` file format, version 1

- Status: accepted (M0, 2026-11-21). This ADR is the source of truth for the code until the spec is synced.

## Context

Designs are user files: shared, versioned in git, opened by later app versions. They must round-trip exactly and be read strictly, as KataBox's imports are.

## Decision

A `.kbarch` file is pretty-printed UTF-8 JSON of `ka_core::Design`, with a final newline:

```text
Design  { schema_version: 1, id, name, problem_id?, nodes[], edges[], groups[], notes[], viewport }
Node    { id, kind, flavour, label, position{x,y}, props{key: value}, group_id? }
Edge    { id, from, to, protocol }        // http | grpc | ws | query | publish | consume | replicate
Group   { id, kind, label, position, size{w,h}, parent_id? }   // kind: region | az
Note    { id, text, position, size }
Viewport{ x, y, zoom }
```

- **`groups`** sit beside `notes`: Region and AZ zones are part of a design. Positions are always **absolute** canvas coordinates, also for cards inside zones. Membership is `Node.group_id` (the innermost zone holding the card's centre) and `Group.parent_id` (an AZ's region).
- **`PropValue` is `number | boolean | string`** (untagged). There is no int/float split: JavaScript can't tell `2` from `2.0`, so an integer prop is a number the registry's `PropType::Int` checks is whole. Whole numbers are written without `.0`, so a save, open, save cycle gives identical bytes.
- **Reading is strict:**
  1. The size is capped at 8 MiB before reading.
  2. `schema_version` is checked first: newer gives `newer_file_schema`, older is migrated step by step (none yet).
  3. Every struct has `deny_unknown_fields`.
  4. The design is then validated: id rules (1–64 chars of `[A-Za-z0-9_-]`, unique across all items), edge endpoints, the wiring table, flavour-for-kind, prop keys, types and ranges, zone references without cycles, text caps, finite numbers, item counts.
- **Writing is atomic:** validate, write `<file>.tmp`, flush, rename over the target. An invalid design is never written.
- **Errors** have stable codes: `invalid_file`, `newer_file_schema`, `invalid_design`, `file_too_large`, `not_found`, `io`, `path_not_allowed`.

## Consequences

A format change bumps `SCHEMA_VERSION` and adds a migration step with a test file of the old version. Hand-edited files with typos are refused with a reason rather than silently fixed.

## Version 2 (M1.0, 2026-11-22)

`SCHEMA_VERSION` is 2. Changes, and how a v1 file is migrated on open (`ka-app/src/kbarch.rs`):

- **Autoscaling is a wrapper.** `GroupKind` gains `autoscale`, and a `Group` may carry
  `scaling: { flavour, props }` (only an autoscale wrapper has one; the field is omitted
  otherwise). The Autoscaling kind keeps its registry entry, with `placement = "wrapper"`, and a
  card of that kind is now invalid (`wrapper_as_card`); a wrapper needs a valid policy
  (`bad_scaling`). Compute cards (app, micro, worker, serverless) inside the frame join it.
  *Migration:* each v1 Autoscaling card becomes a wrapper around the compute cards it was wired
  to (their bounding box, padded 32 px), or a default frame at its own position; its edges are
  dropped, the wrapped cards join it, and its parent is the first wrapped card's old zone.
- **Removed props.** SQL `read_replicas` (replicas are separate cards linked by Replicate edges)
  and CDN `hit_rate` (computed from TTL and the traffic's key skew). *Migration:* dropped.

Each migration rule has its own test with a v1 file.
