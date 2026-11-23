//! `ka-pack` command line (manual argument parsing, as in KataBox):
//!
//! ```text
//! ka-pack new <id>                       scaffold content/problems/<id>/ (with TODO markers)
//! ka-pack validate [--run] [--problems a,b] [--kind problems|samples|guide|practice] [--content DIR]
//! ka-pack build --out FILE [--content DIR]
//! ```
//!
//! `--run` judges each reference (Accepted on every test) and naive design (fails a hidden
//! test) with `ka-judge`, and simulates every sample step with a callout to check its numbers.

use std::path::PathBuf;
use std::process::ExitCode;

use ka_pack::{
    Issue, build_content, load_guide, load_problems, load_samples, validate_guide,
    validate_problems, validate_samples, write_db,
};

const USAGE: &str = "usage:
  ka-pack new <id>
  ka-pack validate [--run [--verbose]] [--problems a,b] [--kind problems|samples|guide|practice] [--content DIR]
  ka-pack build --out FILE [--content DIR]
  ka-pack pack HEAD.toml --problems a,b --out FILE.kbpack [--content DIR]
  ka-pack try --problems a,b [seconds]   judge the designs only (tuning drafts; not a check)
  ka-pack migrate [--content DIR]   rewrite every .kbarch in the current format
  ka-pack challenge [--problems a,b] [--write] [--evals N]   search half-cost proofs (M8.5)";

struct Args {
    content: PathBuf,
    run: bool,
    verbose: bool,
    problems: Option<Vec<String>>,
    kind: Option<String>,
    out: Option<PathBuf>,
    rest: Vec<String>,
}

fn parse(mut it: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut a = Args {
        content: PathBuf::from("content"),
        run: false,
        verbose: false,
        problems: None,
        kind: None,
        out: None,
        rest: Vec::new(),
    };
    while let Some(arg) = it.next() {
        let mut value = |name: &str| it.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--content" => a.content = PathBuf::from(value("--content")?),
            "--run" => a.run = true,
            "--verbose" => a.verbose = true,
            "--problems" => {
                a.problems = Some(value("--problems")?.split(',').map(str::to_owned).collect())
            }
            "--kind" => a.kind = Some(value("--kind")?),
            "--out" => a.out = Some(PathBuf::from(value("--out")?)),
            s if s.starts_with("--") => return Err(format!("unknown option {s}")),
            _ => a.rest.push(arg),
        }
    }
    Ok(a)
}

fn report(issues: &[Issue]) -> ExitCode {
    for i in issues {
        eprintln!("  {i}");
    }
    eprintln!("{} issue(s)", issues.len());
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let mut argv = std::env::args().skip(1);
    let Some(cmd) = argv.next() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let args = match parse(argv) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let registry = ka_core::registry();
    match cmd.as_str() {
        // M8.5: search each problem's "same SLO, half the cost" proof in parallel.
        "challenge" => {
            let write = args.rest.iter().any(|a| a == "--write" || a == "write");
            let cap: usize = args
                .rest
                .iter()
                .find_map(|a| a.strip_prefix("evals=").and_then(|v| v.parse().ok()))
                .unwrap_or(400);
            let mut problems = match load_problems(&args.content, &registry) {
                Ok(p) => p,
                Err(issues) => return report(&issues),
            };
            if let Some(only) = &args.problems {
                problems.retain(|p| only.contains(&p.def.id));
            }
            let found: Vec<Option<ka_pack::challenge::Found>> = std::thread::scope(|s| {
                let handles: Vec<_> = problems
                    .iter()
                    .map(|p| {
                        let r = &registry;
                        s.spawn(move || ka_pack::challenge::search(p, r, cap))
                    })
                    .collect();
                handles
                    .into_iter()
                    .map(|h| h.join().ok().flatten())
                    .collect()
            });
            for (p, f) in problems.iter().zip(&found) {
                let Some(f) = f else {
                    println!("{}: the reference isn't Accepted; skipped", p.def.id);
                    continue;
                };
                println!(
                    "{}: ${:.0} -> ${:.0} ({:.0}%), {} evaluations, ratio {}",
                    f.id,
                    f.ref_cost,
                    f.cost,
                    f.cost / f.ref_cost.max(1e-9) * 100.0,
                    f.evals,
                    f.ratio()
                        .map_or("none (over 80%)".into(), |r| r.to_string())
                );
                if write && let Err(e) = ka_pack::challenge::write(&args.content, p, f, &registry) {
                    eprintln!("{}: {e}", p.def.id);
                }
            }
            ExitCode::SUCCESS
        }
        "new" => {
            let Some(id) = args.rest.first() else {
                eprintln!("{USAGE}");
                return ExitCode::FAILURE;
            };
            match ka_pack::scaffold::new_problem(&args.content, id) {
                Ok(dir) => {
                    println!(
                        "created {} (fill in every {})",
                        dir.display(),
                        ka_pack::TODO_MARKER
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        // Write a `.kbpack` from problem folders, then open it the way the app does (every
        // check, the designs judged), so an author sees what an importer would.
        "pack" => {
            let (Some(only), Some(out), Some(head_path)) =
                (&args.problems, &args.out, args.rest.first())
            else {
                eprintln!("pack needs HEAD.toml --problems a,b --out FILE\n{USAGE}");
                return ExitCode::FAILURE;
            };
            let head: ka_pack::kbpack::PackHead = match std::fs::read_to_string(head_path)
                .map_err(|e| e.to_string())
                .and_then(|s| toml::from_str(&s).map_err(|e| e.to_string()))
            {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("{head_path}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let dirs: Vec<PathBuf> = only
                .iter()
                .map(|id| args.content.join("problems").join(id))
                .collect();
            let sha = match ka_pack::kbpack::write_pack(&dirs, &head, out) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::FAILURE;
                }
            };
            let staging =
                std::env::temp_dir().join(format!("ka-pack-check-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&staging);
            let opened = ka_app::packs::PackOpener::open(
                &ka_pack::kbpack::Opener,
                out,
                &staging,
                &std::collections::BTreeSet::new(),
                env!("CARGO_PKG_VERSION"),
                &registry,
            );
            let _ = std::fs::remove_dir_all(&staging);
            match opened {
                Ok(p) => {
                    println!(
                        "wrote {} ({} problem(s), SHA-256 {sha}); it opens and judges cleanly",
                        out.display(),
                        p.problems.len()
                    );
                    ExitCode::SUCCESS
                }
                Err(issues) => {
                    for i in &issues {
                        eprintln!("  {i}");
                    }
                    eprintln!("wrote {}, but the app would refuse it", out.display());
                    ExitCode::FAILURE
                }
            }
        }
        // Judge only (no text or structure checks), for tuning a problem's designs while its
        // prose is still a draft. It proves nothing: `validate --run` is the check.
        "try" => {
            let Some(only) = &args.problems else {
                eprintln!("try needs --problems a,b\n{USAGE}");
                return ExitCode::FAILURE;
            };
            let mut problems = match load_problems(&args.content, &registry) {
                Ok(p) => p,
                Err(issues) => return report(&issues),
            };
            // M10: `outage--<id>` tries a replay (fix = reference, as it was = naive).
            match ka_pack::outages::load_outages(&args.content, &registry) {
                Ok(outages) => problems.extend(
                    outages
                        .iter()
                        .filter(|o| only.contains(&ka_core::outage::outage_problem_id(&o.def.id)))
                        .map(ka_pack::problems::ProblemDir::from_outage),
                ),
                Err(issues) => {
                    for i in &issues {
                        println!("  {i}");
                    }
                }
            }
            problems.retain(|p| only.contains(&p.def.id));
            // Per card, on the first test: where the money and the load go.
            for p in &problems {
                let Some(t) = p.def.tests.first() else {
                    continue;
                };
                for (name, d) in [("reference", &p.reference), ("naive", &p.naive)] {
                    if let Ok(r) = ka_judge::run_test(&p.def, t, d, &registry) {
                        let cards = r
                            .nodes
                            .iter()
                            .map(|n| {
                                format!(
                                    "{} ${:.0} {:.0}%{}",
                                    n.id,
                                    n.cost_month,
                                    n.utilisation * 100.0,
                                    if n.drops > 0.0 {
                                        format!(" drops {:.0}", n.drops)
                                    } else {
                                        String::new()
                                    }
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        println!("  {name} cards ({}): {cards}", t.id);
                    }
                }
            }
            // `try --problems a seconds`: every test's seconds with errors (or over the p95).
            if args.rest.iter().any(|a| a == "seconds") {
                for p in &problems {
                    for t in &p.def.tests {
                        for (name, d) in [("reference", &p.reference), ("naive", &p.naive)] {
                            let Ok(r) = ka_judge::run_test(&p.def, t, d, &registry) else {
                                continue;
                            };
                            let bad = r
                                .series
                                .iter()
                                .filter(|s| s.errors > 0.0 || s.p95_ms > p.def.nfr.p95_ms)
                                .map(|s| {
                                    format!(
                                        "{}:{:.0}e/{:.0}ms",
                                        s.t,
                                        100.0 * s.errors / s.requests.max(1.0),
                                        s.p95_ms
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join(" ");
                            println!(
                                "  {name} {} seconds (error %/p95): {bad}{}",
                                t.id,
                                if r.conflicts > 0.0 || r.duplicates > 0.0 || r.dead_lettered > 0.0
                                {
                                    format!(
                                        " | conflicts {:.0}, duplicates {:.0}, dead-lettered {:.0}",
                                        r.conflicts, r.duplicates, r.dead_lettered
                                    )
                                } else {
                                    String::new()
                                }
                            );
                        }
                    }
                }
            }
            let issues = ka_pack::run::run_checks(&problems, &registry, true);
            for i in &issues {
                println!("  {i}");
            }
            println!("(try: designs judged only; run validate --run before committing)");
            ExitCode::SUCCESS
        }
        "migrate" => match ka_pack::migrate::rewrite(&args.content, &registry) {
            Ok((changed, total)) => {
                println!("{changed} of {total} .kbarch file(s) rewritten");
                ExitCode::SUCCESS
            }
            Err(issues) => report(&issues),
        },
        "validate" => {
            let kind = args.kind.as_deref();
            if let Some(k) = kind
                && !["problems", "samples", "guide", "practice"].contains(&k)
            {
                eprintln!("--kind {k}: one of problems, samples, guide, practice");
                return ExitCode::FAILURE;
            }
            let mut problems = match load_problems(&args.content, &registry) {
                Ok(p) => p,
                Err(issues) => return report(&issues),
            };
            // Samples and concept cards (the same story format) read the full problem list
            // (their runs name problem tests).
            let check_samples = |problems: &[ka_pack::ProblemDir]| -> Result<usize, ExitCode> {
                let samples = load_samples(&args.content, &registry).map_err(|i| report(&i))?;
                let concepts =
                    ka_pack::load_concepts(&args.content, &registry).map_err(|i| report(&i))?;
                let mut issues = validate_samples(&samples, problems, &registry);
                issues.extend(ka_pack::validate_concepts(&concepts, problems, &registry));
                if issues.is_empty() && args.run {
                    for stories in [&samples, &concepts] {
                        issues.extend(ka_pack::samples::check_callouts(
                            stories,
                            problems,
                            &registry,
                            args.verbose,
                        ));
                    }
                }
                if issues.is_empty() && !concepts.is_empty() {
                    println!("{} concept card(s) valid", concepts.len());
                }
                if issues.is_empty() {
                    Ok(samples.len())
                } else {
                    Err(report(&issues))
                }
            };
            // The Guide and the patterns read the problem list (practice links) and need no
            // simulation.
            let check_guide = |problems: &[ka_pack::ProblemDir]| -> Result<usize, ExitCode> {
                let guide = load_guide(&args.content).map_err(|i| report(&i))?;
                let patterns = ka_pack::patterns::load_patterns(&args.content, &registry)
                    .map_err(|i| report(&i))?;
                let mut issues = validate_guide(&guide, problems, &registry);
                issues.extend(ka_pack::patterns::validate_patterns(
                    &patterns, problems, &registry,
                ));
                let flows = ka_pack::decide::load_decide(&args.content).map_err(|i| report(&i))?;
                let glossary =
                    ka_pack::decide::load_glossary(&args.content).map_err(|i| report(&i))?;
                issues.extend(ka_pack::decide::validate_decide(&flows));
                issues.extend(ka_pack::decide::validate_glossary(
                    &glossary,
                    &ka_pack::decide::corpus(&args.content),
                ));
                if issues.is_empty() {
                    println!(
                        "{} pattern(s), {} decision guide(s), {} glossary term(s) valid",
                        patterns.len(),
                        flows.len(),
                        glossary.len()
                    );
                }
                if !issues.is_empty() {
                    return Err(report(&issues));
                }
                if args.verbose {
                    for (kind, url) in ka_pack::guide::unchecked_sources(&guide) {
                        println!("unchecked source: {kind} {url}");
                    }
                }
                Ok(guide.len())
            };
            // Practice content (numbers, drills, paths): structural, plus `--run` for drills.
            let check_practice = |problems: &[ka_pack::ProblemDir]| -> Result<String, ExitCode> {
                let practice = ka_pack::practice::load_practice(&args.content, &registry)
                    .map_err(|i| report(&i))?;
                let issues = ka_pack::practice::validate_practice(&practice, problems, &registry);
                if !issues.is_empty() {
                    return Err(report(&issues));
                }
                if args.run {
                    let issues =
                        ka_pack::practice::run_practice(&practice, &registry, args.verbose);
                    if !issues.is_empty() {
                        return Err(report(&issues));
                    }
                }
                Ok(ka_pack::practice::summary(&practice))
            };
            if kind == Some("practice") {
                return match check_practice(&problems) {
                    Ok(line) => {
                        println!("{line}");
                        ExitCode::SUCCESS
                    }
                    Err(code) => code,
                };
            }
            if kind == Some("guide") {
                return match check_guide(&problems) {
                    Ok(n) => {
                        println!("{n} guide entr(ies) valid");
                        ExitCode::SUCCESS
                    }
                    Err(code) => code,
                };
            }
            if kind == Some("samples") {
                return match check_samples(&problems) {
                    Ok(n) => {
                        println!(
                            "{n} sample(s) valid{}",
                            if args.run { ", callouts checked" } else { "" }
                        );
                        ExitCode::SUCCESS
                    }
                    Err(code) => code,
                };
            }
            // Everything (no --kind, no --problems): the samples too.
            let (samples, guide) = if kind.is_none() && args.problems.is_none() {
                let s = match check_samples(&problems) {
                    Ok(n) => n,
                    Err(code) => return code,
                };
                let g = match check_guide(&problems) {
                    Ok(n) => n,
                    Err(code) => return code,
                };
                match check_practice(&problems) {
                    Ok(line) => println!("{line}"),
                    Err(code) => return code,
                }
                (Some(s), Some(g))
            } else {
                (None, None)
            };
            if let Some(only) = &args.problems {
                problems.retain(|p| only.contains(&p.def.id));
                if problems.len() != only.len() {
                    eprintln!("some of {only:?} don't exist");
                    return ExitCode::FAILURE;
                }
            }
            let issues = validate_problems(&problems, &registry);
            if !issues.is_empty() {
                return report(&issues);
            }
            if args.run {
                let issues = ka_pack::run::run_checks(&problems, &registry, args.verbose);
                if !issues.is_empty() {
                    return report(&issues);
                }
            }
            println!(
                "{} problem(s) valid{}",
                problems.len(),
                if args.run { ", judged" } else { "" }
            );
            if let Some(n) = samples {
                println!(
                    "{n} sample(s) valid{}",
                    if args.run { ", callouts checked" } else { "" }
                );
            }
            if let Some(n) = guide {
                println!("{n} guide entr(ies) valid");
            }
            ExitCode::SUCCESS
        }
        "build" => {
            let Some(out) = &args.out else {
                eprintln!("build needs --out FILE\n{USAGE}");
                return ExitCode::FAILURE;
            };
            let data = match build_content(&args.content, &registry) {
                Ok(d) => d,
                Err(issues) => return report(&issues),
            };
            match write_db(out, &data) {
                Ok(()) => {
                    println!(
                        "wrote {} ({} problems, {} samples)",
                        out.display(),
                        data.problems.len(),
                        data.samples.len()
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("unknown command {cmd}\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}
