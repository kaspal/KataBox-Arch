//! `ka-pack challenge`: find a "same SLO, half the cost" proof for each problem (M8.5).
//!
//! Starting from the reference, it tries cost-cutting moves in a fixed order (fewer
//! instances, shards, partitions or memory; a cheaper flavour; a card removed; a smaller
//! autoscaling range) and keeps the first move that lowers the first test's cost while every
//! test stays Accepted (budget ignored), until no move helps or the evaluation cap is reached.
//! Everything is deterministic: the judge is seeded per test and the moves come in design
//! order. With `--write` it saves `challenge.kbarch` and, when 50% isn't reached, the proven
//! ratio rounded up to the next 5% as `challenge_ratio`; above 80% the problem has no
//! challenge (review ★17).

use std::path::Path;

use ka_core::{Design, PropValue, Registry};
use ka_judge::{Verdict, judge};

use crate::problems::{CHALLENGE_DEFAULT, CHALLENGE_MAX, ProblemDir};

/// What the search found for one problem.
#[derive(Debug, Clone)]
pub struct Found {
    pub id: String,
    pub ref_cost: f64,
    pub cost: f64,
    pub design: Design,
    pub evals: usize,
}

impl Found {
    /// The ratio the challenge would use (None: above 80%, no challenge).
    pub fn ratio(&self) -> Option<f64> {
        let raw = self.cost / self.ref_cost.max(1e-9);
        let up = (raw / 0.05 - 1e-9).ceil() * 0.05;
        let r = up.max(CHALLENGE_DEFAULT);
        (r <= CHALLENGE_MAX + 1e-9).then_some((r * 100.0).round() / 100.0)
    }
}

/// Cost of the first test if every test is Accepted (budget ignored), else None.
fn evaluate(p: &ProblemDir, d: &Design, registry: &Registry) -> Option<f64> {
    if !ka_core::validate(d, registry).is_empty() {
        return None;
    }
    let mut def = p.def.clone();
    def.nfr.budget_usd = f64::MAX;
    let j = judge(&def, d, registry, 1.0);
    (j.verdict == Verdict::Accepted).then_some(j.cost_month)
}

fn num(d: &Design, i: usize, key: &str, registry: &Registry) -> Option<f64> {
    let n = &d.nodes[i];
    match n.props.get(key) {
        Some(PropValue::Number(v)) => Some(*v),
        Some(_) => None,
        None => {
            let spec = registry.kind(n.kind);
            spec.flavour(n.flavour)
                .and_then(|f| f.overrides.iter().find(|o| o.key == key))
                .map(|o| o.value.clone())
                .or_else(|| spec.prop(key).map(|p| p.default.clone()))
                .and_then(|v| match v {
                    PropValue::Number(v) => Some(v),
                    _ => None,
                })
        }
    }
}

/// A flavour swap is a fair move only when the flavour's price is cited (or is its kind's):
/// a proof must not lean on teaching placeholders the citations will change.
fn price_is_cited(registry: &Registry, kind: ka_core::ComponentKind, f: ka_core::Flavour) -> bool {
    let suffix = format!("@{}", ka_core::flavour_id(f));
    let prefix = format!("{}.cost.", kind.id());
    ka_core::calibration_rows(registry)
        .iter()
        .filter(|r| r.id.starts_with(&prefix) && r.id.ends_with(&suffix))
        .all(|r| ka_core::CITED.contains(&r.id.as_str()))
}

/// Candidate designs one move away, cheapest-looking first within each card.
fn moves(d: &Design, registry: &Registry) -> Vec<Design> {
    let mut out = Vec::new();
    for i in 0..d.nodes.len() {
        let spec = registry.kind(d.nodes[i].kind);
        for key in [
            "instances",
            "shards",
            "partitions",
            "executors",
            "batch_slots",
        ] {
            let Some(cur) = num(d, i, key, registry) else {
                continue;
            };
            let Some(p) = spec.prop(key) else { continue };
            let min = match p.ty {
                ka_core::PropType::Int { min, .. } => min.max(1.0),
                _ => 1.0,
            };
            for next in [(cur / 2.0).floor(), cur - 1.0] {
                if next >= min && next < cur {
                    let mut c = d.clone();
                    c.nodes[i].props.insert(key.into(), PropValue::Number(next));
                    out.push(c);
                }
            }
        }
        if let Some(gb) = num(d, i, "memory_gb", registry)
            && gb > 0.5
            && spec.prop("memory_gb").is_some()
        {
            let mut c = d.clone();
            c.nodes[i]
                .props
                .insert("memory_gb".into(), PropValue::Number((gb / 2.0).max(0.5)));
            out.push(c);
        }
        for f in &spec.flavours {
            if f.flavour != d.nodes[i].flavour
                && price_is_cited(registry, d.nodes[i].kind, f.flavour)
            {
                let mut c = d.clone();
                c.nodes[i].flavour = f.flavour;
                out.push(c);
            }
        }
        // Remove the card and its edges.
        let id = d.nodes[i].id.clone();
        let mut c = d.clone();
        c.nodes.remove(i);
        c.edges.retain(|e| e.from != id && e.to != id);
        out.push(c);
    }
    for g in 0..d.groups.len() {
        let Some(scaling) = &d.groups[g].scaling else {
            continue;
        };
        let auto = registry.kind(ka_core::ComponentKind::Autoscale);
        for key in ["max_instances", "min_instances"] {
            let cur = match scaling.props.get(key) {
                Some(PropValue::Number(v)) => *v,
                _ => match auto.prop(key).map(|p| p.default.clone()) {
                    Some(PropValue::Number(v)) => v,
                    _ => continue,
                },
            };
            for next in [(cur / 2.0).floor(), cur - 1.0] {
                if next >= 1.0 && next < cur {
                    let mut c = d.clone();
                    if let Some(s) = c.groups[g].scaling.as_mut() {
                        s.props.insert(key.into(), PropValue::Number(next));
                    }
                    out.push(c);
                }
            }
        }
    }
    out
}

/// First-improvement search from the reference.
pub fn search(p: &ProblemDir, registry: &Registry, cap: usize) -> Option<Found> {
    let mut current = p.reference.clone();
    let ref_cost = evaluate(p, &current, registry)?;
    let mut cost = ref_cost;
    let mut evals = 1;
    'outer: loop {
        for cand in moves(&current, registry) {
            if evals >= cap {
                break 'outer;
            }
            evals += 1;
            if let Some(c) = evaluate(p, &cand, registry)
                && c < cost - 0.5
            {
                current = cand;
                cost = c;
                continue 'outer;
            }
        }
        break;
    }
    Some(Found {
        id: p.def.id.clone(),
        ref_cost,
        cost,
        design: current,
        evals,
    })
}

/// Save the proof and its ratio (or remove both when the problem has no challenge).
pub fn write(root: &Path, p: &ProblemDir, f: &Found, registry: &Registry) -> Result<(), String> {
    let dir = root.join(&p.dir);
    let toml_path = dir.join("problem.toml");
    let text = std::fs::read_to_string(&toml_path).map_err(|e| e.to_string())?;
    let mut lines: Vec<String> = text
        .lines()
        .filter(|l| !l.starts_with("challenge_ratio"))
        .map(str::to_owned)
        .collect();
    match f.ratio() {
        None => {
            let _ = std::fs::remove_file(dir.join("challenge.kbarch"));
        }
        Some(r) => {
            let mut d = f.design.clone();
            d.id = format!("{}-challenge", p.def.id);
            let bytes = ka_app::kbarch::to_bytes(&d, registry).map_err(|e| e.message)?;
            std::fs::write(dir.join("challenge.kbarch"), bytes).map_err(|e| e.to_string())?;
            if r > CHALLENGE_DEFAULT + 1e-9 {
                let at = lines
                    .iter()
                    .position(|l| l.starts_with('['))
                    .unwrap_or(lines.len());
                lines.insert(at, format!("challenge_ratio = {r}"));
                if at > 0 && !lines[at - 1].trim().is_empty() {
                    lines.insert(at, String::new());
                }
            }
        }
    }
    std::fs::write(&toml_path, lines.join("\n") + "\n").map_err(|e| e.to_string())
}
