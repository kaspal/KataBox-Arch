//! `ka-pack` command line (manual argument parsing, as in KataBox):
//!
//! ```text
//! ka-pack new <id>                       scaffold content/problems/<id>/ (with TODO markers)
//! ka-pack validate [--run] [--problems a,b] [--kind problems] [--content DIR]
//! ka-pack build --out FILE [--content DIR]
//! ```
//!
//! `--run` judges each reference (Accepted on every test) and naive design (fails a hidden
//! test) with `ka-judge`.

use std::path::PathBuf;
use std::process::ExitCode;

use ka_pack::{Issue, build_content, load_problems, validate_problems, write_db};

const USAGE: &str = "usage:
  ka-pack new <id>
  ka-pack validate [--run [--verbose]] [--problems a,b] [--kind problems] [--content DIR]
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
            if let Some(k) = &args.kind
                && k != "problems"
            {
                eprintln!("--kind {k}: only `problems` exists so far (the others arrive in M3)");
                return ExitCode::FAILURE;
            }
            let mut problems = match load_problems(&args.content, &registry) {
                Ok(p) => p,
                Err(issues) => return report(&issues),
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
                    println!("wrote {} ({} problems)", out.display(), data.problems.len());
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
