# Privacy

KataBox Arch runs on your computer and keeps your data there. This page says what it stores, where, and when it
uses the network.

## What KataBox Arch does not do

- No telemetry or analytics, ever.
- No account or sign-in.
- No crash reporting.
- No update checks. The auto-updater is not compiled into the app.
- No ads and no third-party services.

## What is stored, and where

Everything stays on your machine. **`%APPDATA%\dev.katabox.arch`** holds `user.db`, a SQLite database with:

- your settings (appearance, language, canvas and run choices) and the list of recent files;
- drafts: the design you are building for each problem;
- submissions with their verdicts, scores, costs and run reports, and your cost-challenge results;
- problem status (attempted, solved), the hints you unlocked and the days you practised;
- which Guide entries you read, your estimation drills, quiz answers and spaced-review cards;
- the problem packs you installed (their files sit next to it in `packs/`).

The window's size and position are kept in the same folder. Development builds use a separate `-dev` folder.

Your `.kbarch` designs and anything you export (PNG, SVG, video, PDF) are written only where you choose in a save
dialog. When you uninstall, the data folder is kept unless you delete it yourself.

## When KataBox Arch uses the network

It doesn't. Building designs, running the simulation, judging, the Guide, samples, replays and everything else work
offline. Problems, the Guide and the simulator ship with the app.

## In the future

- **AI review** (planned) will be opt-in. It will send your design and its run results only when you start a review,
  only to the AI provider you configure with your own key, and nothing else. It will never change a score: the judge
  stays deterministic.
- **Share links** (planned, off in this version) will put a design into the link itself; the web viewer decodes it
  in your browser. No server stores designs.

## Questions

Open an issue on the repository. For security problems, see [SECURITY.md](SECURITY.md).
