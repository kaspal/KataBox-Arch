//! The KataBox Arch simulator (ADR 0006): a seeded, deterministic discrete-event model of a
//! design under a scenario's traffic. Pure library: no Tauri, no I/O, no wall-clock time,
//! no OS randomness (enforced by this crate's `clippy.toml`).
//!
//! ```text
//! compile(design, registry, scenario, options)   // routes, clusters, sampling: or a lint error
//!   → Engine::new(compiled, seed)                // Clone = a snapshot, for scrubbing
//!   → advance_to(t) / run()
//!   → result()                                   // RunResult, weighted to real traffic
//! ```

pub mod behaviour;
pub mod compile;
pub mod engine;
pub mod hist;
pub mod result;
pub mod route;

use std::sync::Arc;

pub use compile::{Compiled, SAMPLE_CAP_RPS, ScaleGroup, SimError, SimOptions, Zone, compile};
pub use engine::{Engine, NodeSnap, Packet, Snapshot};
pub use result::{NodeResult, Percentiles, RunResult, SecondResult, TypeResult};

use ka_core::scenario::Scenario;
use ka_core::{Design, Registry};

/// Compile and run to the end.
pub fn simulate(
    design: &Design,
    registry: &Registry,
    scenario: &Scenario,
    seed: u64,
    opts: &SimOptions,
) -> Result<RunResult, SimError> {
    let compiled = Arc::new(compile(design, registry, scenario, opts)?);
    let mut engine = Engine::new(compiled, seed);
    engine.run();
    Ok(engine.result())
}
