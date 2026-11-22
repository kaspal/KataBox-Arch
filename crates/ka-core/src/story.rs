//! Stories for the walkthrough player (ADR 0008): the samples (`content/samples/<id>/`), the
//! first-run tour (a sample marked `tour`), and later the concept cards. A story is a list
//! of steps; each step shows one design snapshot, moves the camera, dims what isn't in focus,
//! shows a caption, and may play traffic and call out a number.
//!
//! Callout numbers are written in the content, never computed at build time (review A5);
//! `ka-pack validate --run` simulates every step that has one and checks them.

use serde::{Deserialize, Serialize};

/// Where the camera looks: a point in flow coordinates, centred in the view, and a zoom.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

/// The traffic a step plays: a test of the story's problem, or a Sandbox preset
/// (`normal`, `spike`, `kill_node`) driven by the design's client cards.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct StepRun {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    /// Where the run starts, in simulated seconds (just before the interesting moment).
    #[serde(default)]
    pub start_s: f64,
    /// Playback speed (0.25–8).
    #[serde(default = "one")]
    pub speed: f64,
}

fn one() -> f64 {
    1.0
}

/// The number a callout shows. Each is measured over the whole run, like a test result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum CalloutMetric {
    P95Ms,
    /// 0–1.
    ErrorRate,
    ThroughputRps,
    CostMonth,
}

/// "p95 latency: ~~400 ms~~ 18 ms". `before` is this run on the previous step's design;
/// `after` is this step's. `validate --run` checks both against the simulator.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Callout {
    pub metric: CalloutMetric,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<f64>,
    pub after: f64,
    /// The target to show under the number (the problem's p95, say).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct StoryStep {
    /// A `.kbarch` file in the story's folder.
    pub snapshot: String,
    pub title: String,
    pub title_es: String,
    pub caption: String,
    pub caption_es: String,
    /// Node ids in the spotlight; the rest dims. Empty: nothing dims.
    #[serde(default)]
    pub focus: Vec<String>,
    /// None: fit the design (or the focused cards).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<Camera>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run: Option<StepRun>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callout: Option<Callout>,
}

/// `content/samples/<id>/sample.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct SampleDef {
    pub id: String,
    /// Gallery order.
    pub number: u32,
    pub title: String,
    pub title_es: String,
    pub summary: String,
    pub summary_es: String,
    /// The problem whose tests the steps play (`run.test`), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    /// The first-run tour (offered once from Home).
    #[serde(default)]
    pub tour: bool,
    /// `[[step]]` tables in the TOML.
    #[serde(alias = "step")]
    pub steps: Vec<StoryStep>,
}

impl SampleDef {
    /// The snapshots the steps use, each once, in first-use order.
    pub fn snapshot_files(&self) -> Vec<&str> {
        let mut out: Vec<&str> = Vec::new();
        for s in &self.steps {
            if !out.contains(&s.snapshot.as_str()) {
                out.push(&s.snapshot);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sample_reads_from_toml_and_refuses_unknown_fields() {
        let src = r#"
id = "hello"
number = 1
title = "Hello, Traffic"
title_es = "Hola, tráfico"
summary = "s"
summary_es = "s"
tour = true

[[step]]
snapshot = "one.kbarch"
title = "One server"
title_es = "Un servidor"
caption = "c"
caption_es = "c"
focus = ["app"]
camera = { x = 100, y = 50, zoom = 1.2 }
run = { preset = "normal", start_s = 2 }
callout = { metric = "p95_ms", before = 400, after = 18, target = 80 }

[[step]]
snapshot = "one.kbarch"
title = "Again"
title_es = "Otra vez"
caption = "c"
caption_es = "c"
"#;
        let s: SampleDef = toml::from_str(src).unwrap();
        assert!(s.tour);
        assert_eq!(s.steps.len(), 2);
        assert_eq!(s.snapshot_files(), vec!["one.kbarch"]);
        let run = s.steps[0].run.as_ref().unwrap();
        assert_eq!(
            (run.preset.as_deref(), run.start_s, run.speed),
            (Some("normal"), 2.0, 1.0)
        );
        assert_eq!(s.steps[0].callout.unwrap().metric, CalloutMetric::P95Ms);
        assert!(
            toml::from_str::<SampleDef>(&src.replace("tour = true", "tour = true\nextra = 1"))
                .is_err()
        );
    }
}
