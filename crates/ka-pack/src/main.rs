//! `ka-pack` command line (manual argument parsing, as in KataBox):
//!
//! ```text
//! ka-pack new <id>                       scaffold content/problems/<id>/ (with TODO markers)
//! ka-pack validate [--run] [--problems a,b] [--kind problems|samples|guide] [--content DIR]
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
  ka-pack validate [--run [--verbose]] [--problems a,b] [--kind problems|samples|guide] [--content DIR]
  ka-pack build --out FILE [--content DIR]";

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
        "validate" => {
            let kind = args.kind.as_deref();
            if let Some(k) = kind
                && !["problems", "samples", "guide"].contains(&k)
            {
                eprintln!("--kind {k}: one of problems, samples, guide");
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
