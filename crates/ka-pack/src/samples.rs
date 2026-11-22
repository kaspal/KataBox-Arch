//! Sample folders: `content/samples/<id>/` with `sample.toml` and the `.kbarch` snapshots its
//! steps name. Structural checks here; `check_callouts` (part of `validate --run`) simulates
//! every step with a callout and checks the numbers the content states.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use ka_app::sandbox::{SandboxPreset, sandbox_scenario};
use ka_core::story::{CalloutMetric, SampleDef, StepRun};
use ka_core::{Design, Registry};
use ka_sim::{RunResult, SimOptions};

use crate::problems::ProblemDir;
use crate::{Issue, TODO_MARKER, issue};

#[derive(Debug, Clone)]
pub struct SampleDir {
    /// `samples/<id>` (relative to the content root).
    pub dir: String,
    pub def: SampleDef,
    /// Every `.kbarch` in the folder, by file name.
    pub snapshots: BTreeMap<String, Design>,
    pub toml: String,
}

pub fn load_samples(root: &Path, registry: &Registry) -> Result<Vec<SampleDir>, Vec<Issue>> {
    load_stories(root, "samples", registry)
}

/// Concept cards (M3.5): `content/concepts/<kind>/` in the sample format, one per kind.
pub fn load_concepts(root: &Path, registry: &Registry) -> Result<Vec<SampleDir>, Vec<Issue>> {
    load_stories(root, "concepts", registry)
}

fn load_stories(
    root: &Path,
    folder: &str,
    registry: &Registry,
) -> Result<Vec<SampleDir>, Vec<Issue>> {
    let base = root.join(folder);
    if !base.exists() {
        return Ok(Vec::new());
    }
    let mut dirs: Vec<_> = std::fs::read_dir(&base)
        .map_err(|e| vec![issue(folder, e.to_string())])?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    dirs.sort();
    let mut out = Vec::new();
    let mut issues = Vec::new();
    for name in dirs {
        match load_one(&base.join(&name), &format!("{folder}/{name}"), registry) {
            Ok(s) => out.push(s),
            Err(mut e) => issues.append(&mut e),
        }
    }
    if issues.is_empty() {
        Ok(out)
    } else {
        Err(issues)
    }
}

fn load_one(dir: &Path, rel: &str, registry: &Registry) -> Result<SampleDir, Vec<Issue>> {
    let toml = std::fs::read_to_string(dir.join("sample.toml")).map_err(|e| {
        vec![issue(
            format!("{rel}/sample.toml"),
            format!("missing or unreadable: {e}"),
        )]
    })?;
    let def: SampleDef = toml::from_str(&toml)
        .map_err(|e| vec![issue(format!("{rel}/sample.toml"), e.to_string())])?;
    let mut snapshots = BTreeMap::new();
    let mut issues = Vec::new();
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .map_err(|e| vec![issue(rel, e.to_string())])?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".kbarch"))
        .collect();
    names.sort();
    for name in names {
        let path = format!("{rel}/{name}");
        match std::fs::read(dir.join(&name)) {
            Ok(bytes) => match ka_app::kbarch::parse(&bytes, registry) {
                Ok(d) => {
                    snapshots.insert(name, d);
                }
                Err(e) => issues.push(issue(path, format!("{:?}: {}", e.code, e.message))),
            },
            Err(e) => issues.push(issue(path, e.to_string())),
        }
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(SampleDir {
        dir: rel.into(),
        def,
        snapshots,
        toml,
    })
}

fn preset(name: &str) -> Option<SandboxPreset> {
    serde_json::from_value(serde_json::Value::String(name.into())).ok()
}

/// The player the framing check assumes (px): its caption band, margin and card size match
/// the UI's `stepViewport` (`tour/Player.tsx`), which always fits the focused cards.
const NOMINAL_PLAYER: (f64, f64) = (960.0, 540.0);
const CAPTION_H: f64 = 160.0;
const FRAME_MARGIN: f64 = 48.0;
/// The metric tile's column, kept clear when a step has a callout (review F6).
const CALLOUT_W: f64 = 256.0;
const CARD: (f64, f64) = (208.0, 64.0);
/// Below this zoom the focused cards' text is too small to read.
pub const MIN_FOCUS_ZOOM: f64 = 0.35;

/// The zoom at which a step's focused cards (else every card) fit the nominal player.
fn focus_zoom(focus: &[String], design: &ka_core::Design, callout: bool) -> Option<f64> {
    let lit: Vec<&ka_core::Node> = design
        .nodes
        .iter()
        .filter(|n| focus.is_empty() || focus.contains(&n.id))
        .collect();
    if lit.is_empty() {
        return None;
    }
    let x0 = lit
        .iter()
        .map(|n| n.position.x)
        .fold(f64::INFINITY, f64::min);
    let y0 = lit
        .iter()
        .map(|n| n.position.y)
        .fold(f64::INFINITY, f64::min);
    let x1 = lit
        .iter()
        .map(|n| n.position.x + CARD.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let y1 = lit
        .iter()
        .map(|n| n.position.y + CARD.1)
        .fold(f64::NEG_INFINITY, f64::max);
    let w = NOMINAL_PLAYER.0 - if callout { CALLOUT_W } else { 0.0 } - 2.0 * FRAME_MARGIN;
    let h = NOMINAL_PLAYER.1 - CAPTION_H - 2.0 * FRAME_MARGIN;
    Some((w / (x1 - x0)).min(h / (y1 - y0)).min(1.0))
}

/// Structural checks: ids, text in both languages, snapshots, focus, runs and callouts.
pub fn validate_samples(
    samples: &[SampleDir],
    problems: &[ProblemDir],
    registry: &Registry,
) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut numbers = BTreeSet::new();
    let tours = samples.iter().filter(|s| s.def.tour).count();
    if tours > 1 {
        issues.push(issue(
            "samples",
            format!("{tours} samples are marked `tour`; at most one may be"),
        ));
    }
    for s in samples {
        let d = &s.def;
        let at = |what: &str| format!("{}/{what}", s.dir);
        let file = at("sample.toml");
        if s.dir.rsplit('/').next() != Some(d.id.as_str()) {
            issues.push(issue(
                &file,
                format!("id `{}` doesn't match its folder", d.id),
            ));
        }
        if !numbers.insert(d.number) {
            issues.push(issue(&file, format!("number {} is used twice", d.number)));
        }
        if s.toml.contains(TODO_MARKER) {
            issues.push(issue(&file, format!("unfinished: contains {TODO_MARKER}")));
        }
        for (name, text) in [
            ("title", &d.title),
            ("title_es", &d.title_es),
            ("summary", &d.summary),
            ("summary_es", &d.summary_es),
        ] {
            if text.trim().is_empty() {
                issues.push(issue(&file, format!("`{name}` is empty")));
            }
        }
        let problem = match &d.problem {
            Some(p) => match problems.iter().find(|x| &x.def.id == p) {
                Some(x) => Some(x),
                None => {
                    issues.push(issue(&file, format!("problem `{p}` doesn't exist")));
                    None
                }
            },
            None => None,
        };
        if d.steps.is_empty() {
            issues.push(issue(&file, "a sample needs at least one step"));
        }
        let used: BTreeSet<&str> = d.snapshot_files().into_iter().collect();
        for name in s.snapshots.keys() {
            if !used.contains(name.as_str()) {
                issues.push(issue(at(name), "no step uses this snapshot"));
            }
        }
        for (name, design) in &s.snapshots {
            for i in ka_core::validate(design, registry) {
                issues.push(issue(at(name), format!("invalid design: {i:?}")));
            }
        }
        for (i, step) in d.steps.iter().enumerate() {
            let n = i + 1;
            let here = |m: String| issue(&file, format!("step {n}: {m}"));
            for (name, text) in [
                ("title", &step.title),
                ("title_es", &step.title_es),
                ("caption", &step.caption),
                ("caption_es", &step.caption_es),
            ] {
                if text.trim().is_empty() {
                    issues.push(here(format!("`{name}` is empty")));
                }
            }
            let Some(design) = s.snapshots.get(&step.snapshot) else {
                issues.push(here(format!(
                    "snapshot `{}` isn't in the folder",
                    step.snapshot
                )));
                continue;
            };
            for id in &step.focus {
                if !design.nodes.iter().any(|x| &x.id == id) {
                    issues.push(here(format!(
                        "focus `{id}` isn't a card in {}",
                        step.snapshot
                    )));
                }
            }
            if let Some(zoom) = focus_zoom(&step.focus, design, step.callout.is_some())
                && zoom < MIN_FOCUS_ZOOM
            {
                issues.push(here(format!(
                    "the focused cards are too far apart to read in a {}×{} player (zoom {zoom:.2} < {MIN_FOCUS_ZOOM}); focus fewer cards",
                    NOMINAL_PLAYER.0, NOMINAL_PLAYER.1
                )));
            }
            if let Some(c) = &step.camera
                && !(0.3..=2.0).contains(&c.zoom)
            {
                issues.push(here(format!("camera zoom {} is outside 0.3–2", c.zoom)));
            }
            match &step.run {
                Some(run) => {
                    if let Err(m) = check_run(run, problem) {
                        issues.push(here(m));
                    }
                }
                None if step.callout.is_some() => issues.push(here("a callout needs a run".into())),
                None => {}
            }
            if let Some(c) = &step.callout
                && c.before.is_some()
                && i == 0
            {
                issues.push(here(
                    "the first step has no previous design for `before`".into(),
                ));
            }
        }
    }
    issues
}

fn check_run(run: &StepRun, problem: Option<&ProblemDir>) -> Result<(), String> {
    if !(0.25..=8.0).contains(&run.speed) {
        return Err(format!("speed {} is outside 0.25–8", run.speed));
    }
    if run.start_s < 0.0 {
        return Err("start_s is negative".into());
    }
    match (&run.test, &run.preset) {
        (Some(t), None) => {
            let p = problem.ok_or("`run.test` needs the sample's `problem`")?;
            let test = p
                .def
                .tests
                .iter()
                .find(|x| &x.id == t)
                .ok_or(format!("test `{t}` isn't in {}", p.def.id))?;
            if run.start_s >= test.duration_s {
                return Err(format!("start_s {} is past the test's end", run.start_s));
            }
            Ok(())
        }
        (None, Some(name)) => {
            preset(name).ok_or(format!(
                "unknown preset `{name}` (normal, spike, kill_node)"
            ))?;
            if run.start_s >= ka_app::sandbox::SANDBOX_DURATION_S {
                return Err(format!("start_s {} is past the run's end", run.start_s));
            }
            Ok(())
        }
        _ => Err("a run names exactly one of `test` or `preset`".into()),
    }
}

/// Concept cards: the sample checks, plus each card is named after a component kind and none
/// is a tour.
pub fn validate_concepts(
    concepts: &[SampleDir],
    problems: &[ProblemDir],
    registry: &Registry,
) -> Vec<Issue> {
    let mut issues = validate_samples(concepts, problems, registry);
    for c in concepts {
        let file = format!("{}/sample.toml", c.dir);
        if !registry
            .kinds
            .iter()
            .any(|k| crate::guide::kind_id(k.kind) == c.def.id)
        {
            issues.push(issue(
                &file,
                format!("`{}` isn't a component kind", c.def.id),
            ));
        }
        if c.def.tour {
            issues.push(issue(&file, "a concept card can't be the tour"));
        }
    }
    issues
}

/// A concept card's `content.db` row: the sample row (definition and snapshots) as JSON.
pub fn concept_row(c: &SampleDir, registry: &Registry) -> Result<ka_store::ConceptRow, String> {
    let row = sample_row(c, registry)?;
    Ok(ka_store::ConceptRow {
        kind: row.id.clone(),
        def: serde_json::to_string(&row).map_err(|e| e.to_string())?,
    })
}

/// The `content.db` row: the definition and each snapshot as canonical `.kbarch` JSON.
pub fn sample_row(s: &SampleDir, registry: &Registry) -> Result<ka_store::SampleRow, String> {
    let mut snapshots = BTreeMap::new();
    for (name, d) in &s.snapshots {
        let bytes = ka_app::kbarch::to_bytes(d, registry).map_err(|e| e.message)?;
        snapshots.insert(
            name.clone(),
            String::from_utf8(bytes).map_err(|e| e.to_string())?,
        );
    }
    Ok(ka_store::SampleRow {
        id: s.def.id.clone(),
        number: i64::from(s.def.number),
        def: serde_json::to_string(&s.def).map_err(|e| e.to_string())?,
        snapshots,
    })
}

/// A step's run, simulated the way the player plays it (same scenario, seed and sampling).
pub fn simulate_step(
    design: &Design,
    run: &StepRun,
    problem: Option<&ProblemDir>,
    registry: &Registry,
) -> Result<RunResult, String> {
    if let Some(t) = &run.test {
        let p = problem.ok_or("no problem")?;
        let test = p.def.tests.iter().find(|x| &x.id == t).ok_or("no test")?;
        return ka_judge::run_test(&p.def, test, design, registry).map_err(|e| format!("{e:?}"));
    }
    let preset = run.preset.as_deref().and_then(preset).ok_or("no preset")?;
    let scenario = sandbox_scenario(design, registry, preset).map_err(|e| format!("{e:?}"))?;
    ka_sim::simulate(
        design,
        registry,
        &scenario,
        ka_app::SANDBOX_SEED,
        &SimOptions::default(),
    )
    .map_err(|e| format!("{e:?}"))
}

pub fn metric(r: &RunResult, m: CalloutMetric) -> f64 {
    match m {
        CalloutMetric::P95Ms => r.latency.p95_ms,
        CalloutMetric::ErrorRate => r.error_rate,
        CalloutMetric::ThroughputRps => r.throughput_rps,
        CalloutMetric::CostMonth => r.cost_month,
        CalloutMetric::Lost => r.lost,
    }
}

/// How far a stated number may be from the simulator's: 15%, or a small absolute floor.
fn close(m: CalloutMetric, stated: f64, simulated: f64) -> bool {
    let floor = match m {
        CalloutMetric::P95Ms => 2.0,
        CalloutMetric::ErrorRate => 0.01,
        CalloutMetric::ThroughputRps => 50.0,
        CalloutMetric::CostMonth => 10.0,
        CalloutMetric::Lost => 20.0,
    };
    (stated - simulated).abs() <= (0.15 * simulated.abs()).max(floor)
}

/// `validate --run` for samples: every callout's numbers match the simulator.
pub fn check_callouts(
    samples: &[SampleDir],
    problems: &[ProblemDir],
    registry: &Registry,
    verbose: bool,
) -> Vec<Issue> {
    let mut issues = Vec::new();
    for s in samples {
        let file = format!("{}/sample.toml", s.dir);
        let problem = s
            .def
            .problem
            .as_ref()
            .and_then(|p| problems.iter().find(|x| &x.def.id == p));
        for (i, step) in s.def.steps.iter().enumerate() {
            let (Some(c), Some(run)) = (&step.callout, &step.run) else {
                continue;
            };
            let mut check = |snapshot: &str, stated: f64, which: &str| {
                let design = &s.snapshots[snapshot];
                match simulate_step(design, run, problem, registry) {
                    Ok(r) => {
                        let got = metric(&r, c.metric);
                        if verbose {
                            println!(
                                "{} step {} {which}: {:?} stated {stated}, simulated {got:.4}",
                                s.def.id,
                                i + 1,
                                c.metric
                            );
                        }
                        if !close(c.metric, stated, got) {
                            issues.push(issue(&file, format!("step {}: `{which}` says {stated} but the simulator gives {got:.4}", i + 1)));
                        }
                    }
                    Err(e) => issues.push(issue(&file, format!("step {}: {e}", i + 1))),
                }
            };
            check(&step.snapshot, c.after, "after");
            if let Some(b) = c.before {
                check(&s.def.steps[i - 1].snapshot, b, "before");
            }
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closeness_has_a_relative_band_and_an_absolute_floor() {
        assert!(close(CalloutMetric::P95Ms, 400.0, 360.0));
        assert!(!close(CalloutMetric::P95Ms, 400.0, 300.0));
        assert!(close(CalloutMetric::P95Ms, 3.0, 1.5));
        assert!(close(CalloutMetric::ErrorRate, 0.0, 0.008));
        assert!(!close(CalloutMetric::ErrorRate, 0.6, 0.4));
    }

    #[test]
    fn focused_cards_must_fit_a_readable_zoom() {
        let card = |id: &str, x: f64, y: f64| ka_core::Node {
            id: id.into(),
            kind: ka_core::ComponentKind::App,
            flavour: ka_core::Flavour::Go,
            label: String::new(),
            position: ka_core::Point { x, y },
            props: Default::default(),
            group_id: None,
        };
        let mut d = ka_core::Design::empty("d", "d");
        d.nodes = vec![
            card("a", 0.0, 0.0),
            card("b", 400.0, 0.0),
            card("far", 4000.0, 0.0),
        ];
        let near = focus_zoom(&["a".into(), "b".into()], &d, false).unwrap();
        assert!((MIN_FOCUS_ZOOM..=1.0).contains(&near), "{near}");
        let far = focus_zoom(&["a".into(), "far".into()], &d, false).unwrap();
        // A step with a metric tile has less room, so the same cards fit smaller (F6).
        assert!(focus_zoom(&["a".into(), "far".into()], &d, true).unwrap() < far);
        assert!(far < MIN_FOCUS_ZOOM, "{far}");
        // No focus: the whole design.
        assert_eq!(focus_zoom(&[], &d, false), Some(far));
    }
}
