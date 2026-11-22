//! Live runs behind the commands: one run at a time, controlled by id.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use ka_core::Design;
use ka_sim::{RunResult, SimError, SimOptions, compile};
use serde::{Deserialize, Serialize};

use crate::live::{Control, FrameSink, LiveCore, LiveRun};
use crate::sandbox::{SandboxPreset, sandbox_scenario};
use crate::{App, AppError, AppErrorCode, Result};

/// What the UI needs to read a run's frames: node and edge indices map to these ids.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct RunInfo {
    pub run_id: u32,
    pub duration_s: f64,
    /// The share of requests simulated (1 = all).
    pub sample_factor: f64,
    /// Card ids by frame node index.
    pub nodes: Vec<String>,
    /// Connection ids by frame edge index.
    pub edges: Vec<String>,
    /// Request type ids by frame type index.
    pub request_types: Vec<String>,
    /// The run's chaos (the preset's and any injected), for the timeline's markers.
    pub chaos: Vec<ka_core::scenario::ChaosEvent>,
    /// Where playback starts (an injection resumes from the moment it was made).
    pub start_s: f64,
}

/// What to run: the design, the traffic preset, the speed, chaos the user injected, and
/// where to start playing.
#[derive(Debug, Clone, PartialEq)]
pub struct RunRequest {
    pub preset: SandboxPreset,
    pub speed: f64,
    pub chaos: Vec<ka_core::scenario::ChaosEvent>,
    pub start_s: f64,
}

/// A control sent to the live run.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum RunControl {
    Pause,
    Resume,
    /// Jump to this simulated second (playback pauses there).
    Seek(f64),
    /// 0.25, 0.5, 1, 2, 4 or 8.
    Speed(f64),
    Stop,
}

fn sim_error(e: SimError) -> AppError {
    match e {
        SimError::NoPath { .. } => AppError::new(AppErrorCode::NoPath, e.to_string()),
        SimError::BadScenario(_) => AppError::new(AppErrorCode::Internal, e.to_string()),
    }
}

/// Sandbox runs use one fixed seed, so the same design always plays the same way and an
/// edited design restarts comparably (plan ★13).
pub const SANDBOX_SEED: u64 = 1;

/// The Sandbox's breaking point for `design`: its normal mix, grown until it fails the
/// Sandbox limits (p95 200 ms, 1% errors). Pure and possibly a few seconds long: the command
/// runs it off the async runtime, with a clone of the registry.
pub fn breaking_point(
    design: &Design,
    registry: &ka_core::Registry,
) -> Result<ka_sim::BreakingPoint> {
    if let Some(first) = ka_core::validate(design, registry).first() {
        return Err(AppError::new(
            AppErrorCode::InvalidDesign,
            format!("{first:?}"),
        ));
    }
    let scenario = sandbox_scenario(design, registry, SandboxPreset::Normal).map_err(sim_error)?;
    ka_sim::find_limit(
        design,
        registry,
        &scenario,
        SANDBOX_SEED,
        ka_sim::Limits::default(),
    )
    .map_err(sim_error)
}

impl App {
    /// Start a live Sandbox run of `design`, replacing any run in progress.
    pub fn start_run(
        &self,
        design: &Design,
        request: RunRequest,
        sink: impl FrameSink,
    ) -> Result<RunInfo> {
        let RunRequest {
            preset,
            speed,
            chaos,
            start_s,
        } = request;
        if !start_s.is_finite() || start_s < 0.0 || chaos.len() > 64 {
            return Err(AppError::new(
                AppErrorCode::InvalidInput,
                "start time or chaos list out of range",
            ));
        }
        let issues = self.validate(design);
        if let Some(first) = issues.first() {
            return Err(AppError::new(
                AppErrorCode::InvalidDesign,
                format!("{first:?}"),
            ));
        }
        let mut scenario = sandbox_scenario(design, &self.registry, preset).map_err(sim_error)?;
        scenario.chaos.extend(chaos);
        let compiled = Arc::new(
            compile(design, &self.registry, &scenario, &SimOptions::default())
                .map_err(sim_error)?,
        );
        let info = RunInfo {
            run_id: self.next_run.fetch_add(1, Ordering::Relaxed),
            duration_s: scenario.duration_s,
            sample_factor: compiled.sample,
            nodes: compiled.nodes.iter().map(|n| n.id.clone()).collect(),
            edges: design.edges.iter().map(|e| e.id.clone()).collect(),
            request_types: compiled.types.iter().map(|t| t.def.id.clone()).collect(),
            chaos: scenario.chaos.clone(),
            start_s: start_s.min(scenario.duration_s),
        };
        // Stop the old run first (its thread joins on drop), outside the lock.
        let old = self.live_slot().take();
        drop(old);
        let mut core = LiveCore::new(compiled, design, SANDBOX_SEED);
        if info.start_s > 0.0 {
            core.seek((info.start_s * 1e6) as u64);
        }
        let run = LiveRun::start(core, sink, speed);
        *self.live_slot() = Some((info.run_id, run));
        Ok(info)
    }

    pub fn control_run(&self, run_id: u32, control: RunControl) -> Result<()> {
        let mut slot = self.live_slot();
        if !matches!(slot.as_ref(), Some((id, _)) if *id == run_id) {
            return Err(AppError::new(
                AppErrorCode::NotFound,
                format!("run {run_id}"),
            ));
        }
        let c = match control {
            RunControl::Pause => Control::Pause,
            RunControl::Resume => Control::Resume,
            RunControl::Seek(t) if t.is_finite() => Control::Seek(t),
            RunControl::Speed(s) if s.is_finite() && s > 0.0 => Control::Speed(s),
            RunControl::Stop => {
                let old = slot.take();
                drop(slot);
                drop(old);
                return Ok(());
            }
            other => {
                return Err(AppError::new(
                    AppErrorCode::InvalidInput,
                    format!("{other:?}"),
                ));
            }
        };
        if let Some((_, run)) = slot.as_ref() {
            run.control(c);
        }
        Ok(())
    }

    /// The finished run's full result (None while it is still playing).
    pub fn run_result(&self, run_id: u32) -> Result<Option<RunResult>> {
        match self.live_slot().as_ref() {
            Some((id, run)) if *id == run_id => Ok(run.result()),
            _ => Err(AppError::new(
                AppErrorCode::NotFound,
                format!("run {run_id}"),
            )),
        }
    }

    fn live_slot(&self) -> std::sync::MutexGuard<'_, Option<(u32, LiveRun)>> {
        self.live.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::{Sender, channel};
    use std::time::Duration;

    use super::*;
    use crate::AppPaths;
    use crate::testutil::temp_dir;

    struct Collect(Sender<Vec<u8>>);
    impl FrameSink for Collect {
        fn send(&mut self, frame: Vec<u8>) -> bool {
            self.0.send(frame).is_ok()
        }
    }

    fn req(preset: SandboxPreset, speed: f64) -> RunRequest {
        RunRequest {
            preset,
            speed,
            chaos: Vec::new(),
            start_s: 0.0,
        }
    }

    async fn app() -> App {
        App::open(AppPaths {
            user_db: temp_dir("runs").join("user.db"),
            dev_data: true,
            content_db: None,
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn a_run_starts_streams_and_stops_by_id() {
        let app = app().await;
        let design = crate::kbarch::tests::sample();
        let (tx, rx) = channel();
        let info = app
            .start_run(&design, req(SandboxPreset::Normal, 8.0), Collect(tx))
            .unwrap();
        assert_eq!(info.nodes.len(), design.nodes.len());
        assert_eq!(info.edges.len(), design.edges.len());
        assert_eq!(info.request_types, vec!["read", "write"]);
        let f = rx.recv_timeout(Duration::from_secs(5)).expect("a frame");
        assert_eq!(&f[0..4], b"KAF1");
        assert_eq!(
            app.control_run(info.run_id + 1, RunControl::Pause)
                .unwrap_err()
                .code,
            AppErrorCode::NotFound
        );
        app.control_run(info.run_id, RunControl::Speed(2.0))
            .unwrap();
        assert!(app.run_result(info.run_id).unwrap().is_none());
        app.control_run(info.run_id, RunControl::Stop).unwrap();
        assert_eq!(
            app.run_result(info.run_id).unwrap_err().code,
            AppErrorCode::NotFound
        );
    }

    #[tokio::test]
    async fn a_new_run_replaces_the_old_one() {
        let app = app().await;
        let design = crate::kbarch::tests::sample();
        let (tx1, rx1) = channel();
        let first = app
            .start_run(&design, req(SandboxPreset::Normal, 1.0), Collect(tx1))
            .unwrap();
        let (tx2, _rx2) = channel();
        let second = app
            .start_run(&design, req(SandboxPreset::Spike, 1.0), Collect(tx2))
            .unwrap();
        assert_ne!(first.run_id, second.run_id);
        // The first run's thread has stopped: its channel drains, then closes.
        while rx1.recv_timeout(Duration::from_secs(5)).is_ok() {}
        assert_eq!(
            app.control_run(first.run_id, RunControl::Pause)
                .unwrap_err()
                .code,
            AppErrorCode::NotFound
        );
    }

    #[tokio::test]
    async fn injected_chaos_joins_the_run_which_starts_where_it_was_injected() {
        let app = app().await;
        let design = crate::kbarch::tests::sample();
        let (tx, rx) = channel();
        let kill = ka_core::scenario::ChaosEvent::Kill {
            target: ka_core::scenario::Selector {
                id: Some("app-1".into()),
                ..Default::default()
            },
            at: 12.0,
            for_s: None,
        };
        let info = app
            .start_run(
                &design,
                RunRequest {
                    preset: SandboxPreset::Normal,
                    speed: 1.0,
                    chaos: vec![kill.clone()],
                    start_s: 12.0,
                },
                Collect(tx),
            )
            .unwrap();
        assert_eq!(info.chaos, vec![kill]);
        assert_eq!(info.start_s, 12.0);
        let f = rx.recv_timeout(Duration::from_secs(5)).expect("a frame");
        let t_ms = u32::from_le_bytes([f[10], f[11], f[12], f[13]]);
        assert!(t_ms >= 12_000, "{t_ms}");
    }

    #[test]
    fn the_breaking_point_names_the_first_card_to_give_way() {
        let design = crate::kbarch::tests::sample();
        let b = breaking_point(&design, &ka_core::registry()).unwrap();
        assert!(b.limit_rps > b.base_rps * 0.25, "{b:?}");
        let mut no_path = design.clone();
        no_path.edges.clear();
        assert_eq!(
            breaking_point(&no_path, &ka_core::registry())
                .unwrap_err()
                .code,
            AppErrorCode::NoPath
        );
    }

    #[tokio::test]
    async fn a_design_without_a_path_is_refused_with_no_path() {
        let app = app().await;
        let mut design = crate::kbarch::tests::sample();
        design.edges.clear();
        let (tx, _rx) = channel();
        let err = app
            .start_run(&design, req(SandboxPreset::Normal, 1.0), Collect(tx))
            .unwrap_err();
        assert_eq!(err.code, AppErrorCode::NoPath);
    }
}
