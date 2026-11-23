//! Headless frames for animated export (M7b.2): the live run's `KAF1` frames, stepped at a
//! fixed frame rate as fast as the engine goes (no wall clock), so the same request always
//! gives the same bytes.
//!
//! Hidden tests (review A7): only through a stored submission, with the design and the
//! verdict from the store (never from the UI), only a test that failed, and only the window
//! Replay shows. So an export never shows more than Results already does.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use ka_core::Design;
use ka_sim::{SimOptions, compile};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppErrorCode};
use crate::live::LiveCore;
use crate::runs::{SANDBOX_SEED, sim_error};
use crate::sandbox::{SandboxPreset, sandbox_scenario};
use crate::{App, Result};

/// The longest export (WebM, A7); GIF asks for at most 20 s.
pub const MAX_SECONDS: f64 = 60.0;
/// The frame rates an export may use.
pub const FPS: [u32; 2] = [15, 30];

/// Which run to export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum FramesSource {
    /// The Sandbox design under a preset.
    Sandbox {
        design: Design,
        preset: SandboxPreset,
    },
    /// A design under one of a problem's visible tests.
    Test {
        design: Design,
        problem_id: String,
        test_id: String,
    },
    /// A stored submission under one of its failed tests, hidden ones included, within the
    /// window Replay shows.
    Submission { submission_id: i64, test_id: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FramesRequest {
    pub source: FramesSource,
    pub from_s: f64,
    pub seconds: f64,
    pub fps: u32,
}

/// What the frames refer to: node indices are `nodes`, edge indices are `edges`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FramesInfo {
    pub duration_s: f64,
    pub sample_factor: f64,
    pub nodes: Vec<String>,
    pub edges: Vec<String>,
    pub fps: u32,
    pub from_s: f64,
    /// The design the frames show (for a submission, the stored one).
    pub design: Design,
}

fn refused(msg: &str) -> AppError {
    AppError::new(AppErrorCode::ExportHiddenRefused, msg)
}

/// `info` as JSON, then the frames: `u32 info_len | info | u32 n | (u32 len | frame)*`,
/// little-endian.
pub fn pack(info: &FramesInfo, frames: &[Vec<u8>]) -> Vec<u8> {
    let json = serde_json::to_vec(info).expect("frames info serialises");
    let total: usize = frames.iter().map(|f| f.len() + 4).sum();
    let mut out = Vec::with_capacity(8 + json.len() + total);
    out.extend_from_slice(&(json.len() as u32).to_le_bytes());
    out.extend_from_slice(&json);
    out.extend_from_slice(&(frames.len() as u32).to_le_bytes());
    for f in frames {
        out.extend_from_slice(&(f.len() as u32).to_le_bytes());
        out.extend_from_slice(f);
    }
    out
}

/// One frame every `1/fps` s from `from_s`, `seconds` long, playing at speed 1.
pub fn step_frames(mut core: LiveCore, from_s: f64, seconds: f64, fps: u32) -> Vec<Vec<u8>> {
    if from_s > 0.0 {
        core.seek((from_s * 1e6) as u64);
    }
    let n = (seconds * f64::from(fps)).round() as usize;
    let dt = 1_000_000 / u64::from(fps);
    (0..n).map(|_| core.step(dt, true, 1.0)).collect()
}

impl App {
    /// The packed frames for an export (see `pack`). One export at a time.
    pub async fn run_frames(&self, req: FramesRequest) -> Result<Vec<u8>> {
        let FramesRequest {
            source,
            from_s,
            seconds,
            fps,
        } = req;
        if !FPS.contains(&fps)
            || !from_s.is_finite()
            || from_s < 0.0
            || !seconds.is_finite()
            || seconds <= 0.0
            || seconds > MAX_SECONDS
        {
            return Err(AppError::new(
                AppErrorCode::InvalidInput,
                "frame rate, start or length out of range",
            ));
        }
        let (design, scenario, seed, opts, window) = match source {
            FramesSource::Sandbox { design, preset } => {
                let s = sandbox_scenario(&design, &self.registry, preset).map_err(sim_error)?;
                (design, s, SANDBOX_SEED, SimOptions::default(), None)
            }
            FramesSource::Test {
                design,
                problem_id,
                test_id,
            } => {
                let def = self.problem_def(&problem_id).await?;
                let test = def.test(&test_id).ok_or_else(|| {
                    AppError::new(AppErrorCode::NotFound, format!("test {test_id}"))
                })?;
                if test.hidden {
                    return Err(refused("a hidden test exports only from its Results"));
                }
                let s = def.scenario(test);
                let seed = ka_judge::test_seed(&def.id, &test.id);
                (design, s, seed, ka_judge::options(&def), None)
            }
            FramesSource::Submission {
                submission_id,
                test_id,
            } => {
                let sub =
                    self.user.submission(submission_id).await?.ok_or_else(|| {
                        AppError::new(AppErrorCode::NotFound, "no such submission")
                    })?;
                let design: Design = serde_json::from_str(&sub.design)
                    .map_err(|e| AppError::new(AppErrorCode::Internal, e.to_string()))?;
                let judged: ka_judge::Judgement = serde_json::from_str(&sub.results)
                    .map_err(|e| AppError::new(AppErrorCode::Internal, e.to_string()))?;
                let result = judged
                    .tests
                    .iter()
                    .find(|t| t.id == test_id)
                    .ok_or_else(|| refused("the submission has no verdict for that test"))?;
                // Replay's window: from where it starts to the end of the test. Accepted
                // tests have none, so they can't be exported this way.
                let start = result.replay_at_s.ok_or_else(|| {
                    refused("only a test that failed can be exported from Results")
                })?;
                let def = self.problem_def(&sub.summary.problem_id).await?;
                let test = def.test(&test_id).ok_or_else(|| {
                    AppError::new(AppErrorCode::NotFound, format!("test {test_id}"))
                })?;
                let s = def.scenario(test);
                let seed = ka_judge::test_seed(&def.id, &test.id);
                (design, s, seed, ka_judge::options(&def), Some(start))
            }
        };
        if let Some(start) = window
            && (from_s < start - 1e-9 || from_s + seconds > scenario.duration_s + 1e-9)
        {
            return Err(refused(
                "an export from Results stays inside Replay's window",
            ));
        }
        if from_s + seconds > scenario.duration_s + ka_sim::DRAIN_US as f64 / 1e6 {
            return Err(AppError::new(
                AppErrorCode::InvalidInput,
                "the export runs past the end of the test",
            ));
        }
        if let Some(first) = self.validate(&design).first() {
            return Err(AppError::new(
                AppErrorCode::InvalidDesign,
                format!("{first:?}"),
            ));
        }
        if self.exporting.swap(true, Ordering::AcqRel) {
            return Err(AppError::new(
                AppErrorCode::ExportBusy,
                "an export is already being prepared",
            ));
        }
        let registry = self.registry.clone();
        let job = tokio::task::spawn_blocking(move || {
            let compiled =
                Arc::new(compile(&design, &registry, &scenario, &opts).map_err(sim_error)?);
            let info = FramesInfo {
                duration_s: scenario.duration_s,
                sample_factor: compiled.sample,
                nodes: compiled.nodes.iter().map(|n| n.id.clone()).collect(),
                edges: design.edges.iter().map(|e| e.id.clone()).collect(),
                fps,
                from_s,
                design: design.clone(),
            };
            let core = LiveCore::new(compiled, &design, seed);
            Ok(pack(&info, &step_frames(core, from_s, seconds, fps)))
        })
        .await;
        self.exporting.store(false, Ordering::Release);
        job.map_err(|e| AppError::new(AppErrorCode::Internal, e.to_string()))?
    }
}
