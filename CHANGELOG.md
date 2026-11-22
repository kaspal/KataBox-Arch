# Changelog

All notable changes to KataBox Arch are recorded in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Autoscaling groups** are frames around compute cards, with a scaling policy (flavour,
  minimum and maximum instances, target utilisation, scale-out delay) edited in the Inspector.
- Edges that share a side of a card spread out along it instead of meeting at one point.

### Changed
- Design files are format version 2. Version 1 files still open: an Autoscaling card becomes a
  frame around what it scaled, and two retired properties (SQL read replicas, CDN hit rate) are
  dropped.
- The Inspector's Label field shows the product name as its placeholder.
- "Fit the design" never zooms out below 80%, so card text stays readable.

### Fixed
- A layout loop that re-measured the canvas every frame whenever zones or notes were on screen.
- React Flow's "parent container needs a width and a height" warning when opening the Sandbox.
