//! What a simulation runs: the traffic (request types and their routes, the load curve),
//! external providers, chaos events and pass criteria. Problems define these in
//! `problem.toml`; the Sandbox builds one from its presets (ADR 0006, ADR 0008).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::registry::ComponentKind;

/// One kind of request a problem's traffic is made of (e.g. `redirect`, `create`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct RequestType {
    pub id: String,
    /// Spanish display name (the id doubles as the English one when `name` is absent).
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub name_es: Option<String>,
    /// Share of the problem's traffic, 0..=1 (the shares of a scenario sum to 1).
    pub share: f64,
    /// The route policy (ADR 0006): steps like `"edge?"`, `"compute"`, `"cache?"`,
    /// `"datastore:write"`, `"broker"`, `"worker"`, `"external:email"`, `"compute|datastore"`.
    pub route: Vec<String>,
    /// Distinct keys (URLs, users, pastes…). Keys are ranked by popularity.
    #[serde(default = "default_key_space")]
    pub key_space: u64,
    #[serde(default)]
    pub key_skew: Option<KeySkew>,
    /// A CDN may serve it.
    #[serde(default)]
    pub cacheable: bool,
    /// Request payload, which slows databases that store it.
    #[serde(default = "default_payload_kb")]
    pub payload_kb: f64,
    /// Size of a cached value, which sets how many keys a cache holds.
    #[serde(default = "default_value_kb")]
    pub value_kb: f64,
    /// API keys a rate limiter buckets by (none: one shared key).
    #[serde(default)]
    pub api_keys: Option<u32>,
    /// Traffic a rate limiter should shed: its 429s count as success, not errors.
    #[serde(default)]
    pub abusive: bool,
    /// Async work per message (a post fanned out to followers).
    #[serde(default)]
    pub fanout: Option<Fanout>,
    /// Which latency the problem's p95 target applies to.
    #[serde(default)]
    pub measure: Measure,
    #[serde(default)]
    pub source: Source,
    /// S7: a share of these requests check, then write, one of a few hot keys (the last
    /// room, a promo code's counter): overlapping check-then-write windows are races.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contended: Option<Contended>,
    /// What the type is (M8): requests (the default), or long-lived connections (S1).
    #[serde(default, skip_serializing_if = "TypeKind::is_request")]
    pub kind: TypeKind,
    /// S1 connection types: connections the clients keep open (at a traffic multiplier of
    /// 1), reached over `ramp_s` seconds from the start (0: connected from the start), and the
    /// messages the server pushes to each per second (scaled by the traffic curve).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub connected: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub ramp_s: f64,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub push_rps: f64,
    /// S2: the tokens a request sends to a model and gets back (lognormal).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_in: Option<Tokens>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_out: Option<Tokens>,
    /// S3: the share of these requests that are near-repeats of an earlier one (what a
    /// semantic cache can answer). The traffic declares it; a design can't raise it.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub semantic_repeat_share: f64,
}

/// S2: a token count's mean and 95th percentile.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Tokens {
    pub mean: f64,
    pub p95: f64,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// What a request type stands for (M8).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum TypeKind {
    /// Requests arriving at `share` of the problem's rate (everything before M8).
    #[default]
    Request,
    /// S1: long-lived connections to a WebSocket gateway or IoT broker.
    Connection,
}

impl TypeKind {
    pub fn is_request(&self) -> bool {
        *self == TypeKind::Request
    }
}

/// S7: `share` of a type's requests hit one of `keys` hot keys.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Contended {
    pub keys: u64,
    pub share: f64,
}

fn default_key_space() -> u64 {
    1_000_000
}
fn default_payload_kb() -> f64 {
    1.0
}
fn default_value_kb() -> f64 {
    1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct KeySkew {
    /// Zipf exponent: 0 is uniform; ~1.1 is typical web popularity; higher is hotter.
    pub zipf: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Fanout {
    /// Mean work units per message.
    pub mean: f64,
    /// Share of messages that are heavy (a celebrity post), and their multiplier.
    #[serde(default)]
    pub heavy_share: f64,
    #[serde(default = "one")]
    pub heavy_x: f64,
}

fn one() -> f64 {
    1.0
}

/// Response latency (until the client gets its answer, or the publish is acknowledged), or
/// completion latency (until the last async step is done).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Measure {
    #[default]
    Response,
    Completion,
}

/// Where requests start: client cards, or a scheduler card (cron jobs, crawls).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Source {
    #[default]
    Client,
    Scheduler,
}

/// A provider outside the design (email, SMS, payment, the web), called by an
/// `external:<name>` route step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct External {
    pub name: String,
    pub median_ms: f64,
    pub p99_ms: f64,
    /// Calls per second it accepts; more fail (429). None: unlimited.
    #[serde(default)]
    pub rate_limit: Option<f64>,
}

/// The load over time, as a multiplier of the peak rate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Curve {
    Flat(f64),
    Spike {
        at: f64,
        x: f64,
        #[serde(rename = "for")]
        for_s: f64,
    },
    /// Linear from `from` at t = 0 to `to` at the end.
    Ramp {
        from: f64,
        to: f64,
    },
    /// Piecewise constant: `[[t, x], …]`, sorted by t.
    Steps(Vec<(f64, f64)>),
    /// A spike of `x` lasting `for` seconds every `every` seconds, from t = 0.
    Hourly {
        every: f64,
        x: f64,
        #[serde(rename = "for")]
        for_s: f64,
    },
}

impl Default for Curve {
    fn default() -> Self {
        Curve::Flat(1.0)
    }
}

impl Curve {
    /// The multiplier at time `t` (seconds) of a run lasting `duration` seconds.
    pub fn at(&self, t: f64, duration: f64) -> f64 {
        match self {
            Curve::Flat(x) => *x,
            Curve::Spike { at, x, for_s } => {
                if t >= *at && t < at + for_s {
                    *x
                } else {
                    1.0
                }
            }
            Curve::Ramp { from, to } => {
                from + (to - from) * (t / duration.max(1e-9)).clamp(0.0, 1.0)
            }
            Curve::Steps(steps) => steps
                .iter()
                .rev()
                .find(|(st, _)| t >= *st)
                .map_or(1.0, |(_, x)| *x),
            Curve::Hourly { every, x, for_s } => {
                if every.is_finite() && *every > 0.0 && (t % every) < *for_s {
                    *x
                } else {
                    1.0
                }
            }
        }
    }

    /// The highest multiplier the curve reaches.
    pub fn peak(&self) -> f64 {
        match self {
            Curve::Flat(x) => *x,
            Curve::Spike { x, .. } | Curve::Hourly { x, .. } => x.max(1.0),
            Curve::Ramp { from, to } => from.max(*to),
            Curve::Steps(steps) => steps.iter().map(|(_, x)| *x).fold(1.0, f64::max),
        }
    }
}

/// Which card(s) a chaos event hits.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Selector {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub kind: Option<ComponentKind>,
    /// For data clusters: the current primary, a replica, or any member.
    #[serde(default)]
    pub role: Option<Role>,
    /// Every matching card instead of the first (in id order).
    #[serde(default)]
    pub all: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Primary,
    Replica,
    Any,
}

/// Something going wrong at time `at` (seconds), optionally for `for` seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ChaosEvent {
    /// The card stops answering (a crashed node, a lost cache).
    Kill {
        #[serde(flatten)]
        target: Selector,
        at: f64,
        #[serde(default, rename = "for")]
        for_s: Option<f64>,
    },
    /// Every request through the card takes `add_ms` longer.
    Latency {
        #[serde(flatten)]
        target: Selector,
        add_ms: f64,
        at: f64,
        #[serde(default, rename = "for")]
        for_s: Option<f64>,
    },
    /// The zone (Region or AZ label or id) is lost: every card in it goes down.
    Partition {
        zone: String,
        at: f64,
        #[serde(default, rename = "for")]
        for_s: Option<f64>,
    },
    /// A network split: the zone (Region or AZ label or id) keeps running, but nothing
    /// crosses its border (calls, hops, publishes, replication) until it heals.
    Split {
        zone: String,
        at: f64,
        #[serde(default, rename = "for")]
        for_s: Option<f64>,
    },
    /// Traffic multiplied by `x`.
    Traffic {
        x: f64,
        at: f64,
        #[serde(default, rename = "for")]
        for_s: Option<f64>,
    },
    /// A cache or CDN is emptied (a restart, a purge). `cache_flush` is the same event.
    #[serde(alias = "cache_flush")]
    Flush {
        #[serde(flatten)]
        target: Selector,
        at: f64,
    },
    /// An external provider goes down, slows by `add_ms`, or cuts its rate limit to `rate`
    /// calls per second.
    External {
        name: String,
        at: f64,
        #[serde(default, rename = "for")]
        for_s: Option<f64>,
        #[serde(default)]
        down: bool,
        #[serde(default)]
        add_ms: f64,
        #[serde(default)]
        rate: Option<f64>,
    },
    /// S8: a bad build is deployed to the compute cards selected. Its instances fail
    /// `error_rate` of their requests and add `add_ms`. It rolls out by each card's
    /// `rollout_percent` every `bake_s`, rolls back by itself with `auto_rollback`, and is
    /// rolled back by hand after `for` seconds.
    Deploy {
        #[serde(flatten)]
        target: Selector,
        at: f64,
        #[serde(default)]
        error_rate: f64,
        #[serde(default)]
        add_ms: f64,
        #[serde(default, rename = "for")]
        for_s: Option<f64>,
    },
    /// S8: a bad config change reaches every service. A Config card with `staged` sends it
    /// one Region at a time, every `bake_s`, and halts at the first stage that breaks; with
    /// `kill_switch` the change is rolled back a second after it is noticed.
    ConfigPush {
        at: f64,
        #[serde(default)]
        error_rate: f64,
        #[serde(default)]
        add_ms: f64,
        #[serde(default, rename = "for")]
        for_s: Option<f64>,
    },
    /// S8: DNS can't answer. Clients keep using answers they cached within the TTL of the
    /// DNS card on their path; the rest can't resolve.
    DnsOutage {
        at: f64,
        #[serde(default, rename = "for")]
        for_s: Option<f64>,
    },
    /// S8: the cloud control plane is down: no scaling decisions apply and no replacement
    /// instances start, while running instances keep serving.
    ControlPlane {
        at: f64,
        #[serde(default, rename = "for")]
        for_s: Option<f64>,
    },
}

impl ChaosEvent {
    pub fn at(&self) -> f64 {
        match self {
            ChaosEvent::Kill { at, .. }
            | ChaosEvent::Latency { at, .. }
            | ChaosEvent::Partition { at, .. }
            | ChaosEvent::Split { at, .. }
            | ChaosEvent::Traffic { at, .. }
            | ChaosEvent::Flush { at, .. }
            | ChaosEvent::External { at, .. }
            | ChaosEvent::Deploy { at, .. }
            | ChaosEvent::ConfigPush { at, .. }
            | ChaosEvent::DnsOutage { at, .. }
            | ChaosEvent::ControlPlane { at, .. } => *at,
        }
    }
}

/// What a passing run needs. Unset limits aren't checked.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct PassCriteria {
    #[serde(default)]
    pub p95_ms: Option<f64>,
    /// Judged only on tests with at least `P99_MIN_SAMPLES` completed requests of the
    /// measured types (review A4); `ka-pack` refuses a problem whose tests have fewer.
    #[serde(default)]
    pub p99_ms: Option<f64>,
    #[serde(default)]
    pub error_rate: Option<f64>,
    #[serde(default)]
    pub availability: Option<f64>,
    #[serde(default)]
    pub stale_rate: Option<f64>,
    /// Messages that may be lost (usually 0).
    #[serde(default)]
    pub lost: Option<f64>,
    /// S8 metastable check: from this second to the end of the test, every second must be
    /// within the test's error and p95 limits (the system recovered after its trigger).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovered_by_s: Option<f64>,
    /// S7: writes that raced or landed on a stale primary (usually 0). Tests that set it
    /// must run unsampled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflicts: Option<f64>,
    /// S1: the least share of the target connections that must be connected in every
    /// second after `ramp_s` + 10 s.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connected_min: Option<f64>,
    /// S2: time to first token, 95th percentile, of the requests that call a model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttft_p95_ms: Option<f64>,
}

/// Completed requests a test needs before its p99 is judged (review A4).
pub const P99_MIN_SAMPLES: f64 = 1000.0;

impl PassCriteria {
    /// `self` with every limit set in `over` replacing it (a test's overrides).
    pub fn merged(&self, over: &PassCriteria) -> PassCriteria {
        PassCriteria {
            p95_ms: over.p95_ms.or(self.p95_ms),
            p99_ms: over.p99_ms.or(self.p99_ms),
            error_rate: over.error_rate.or(self.error_rate),
            availability: over.availability.or(self.availability),
            stale_rate: over.stale_rate.or(self.stale_rate),
            lost: over.lost.or(self.lost),
            recovered_by_s: over.recovered_by_s.or(self.recovered_by_s),
            conflicts: over.conflicts.or(self.conflicts),
            connected_min: over.connected_min.or(self.connected_min),
            ttft_p95_ms: over.ttft_p95_ms.or(self.ttft_p95_ms),
        }
    }
}

/// Everything one simulation run needs besides the design and the seed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// Requests per second at a multiplier of 1. 0: use the client cards' `rps` props.
    pub peak_rps: f64,
    pub duration_s: f64,
    pub request_types: Vec<RequestType>,
    #[serde(default)]
    pub externals: Vec<External>,
    #[serde(default)]
    pub traffic: Curve,
    #[serde(default)]
    pub chaos: Vec<ChaosEvent>,
    /// Per request type multipliers for this run (a test that adds abusive traffic).
    #[serde(default)]
    pub type_multipliers: BTreeMap<String, f64>,
    /// A test may make keys hotter than the problem's default.
    #[serde(default)]
    pub key_skew_override: Option<KeySkew>,
    /// The cache and CDN start empty instead of warm.
    #[serde(default)]
    pub cold_caches: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curves() {
        let spike = Curve::Spike {
            at: 20.0,
            x: 4.0,
            for_s: 15.0,
        };
        assert_eq!(spike.at(10.0, 60.0), 1.0);
        assert_eq!(spike.at(20.0, 60.0), 4.0);
        assert_eq!(spike.at(35.0, 60.0), 1.0);
        assert_eq!(spike.peak(), 4.0);
        let ramp = Curve::Ramp { from: 1.0, to: 3.0 };
        assert_eq!(ramp.at(30.0, 60.0), 2.0);
        let steps = Curve::Steps(vec![(0.0, 1.0), (10.0, 2.0), (20.0, 0.5)]);
        assert_eq!(
            (
                steps.at(5.0, 60.0),
                steps.at(15.0, 60.0),
                steps.at(25.0, 60.0)
            ),
            (1.0, 2.0, 0.5)
        );
        let hourly = Curve::Hourly {
            every: 20.0,
            x: 10.0,
            for_s: 2.0,
        };
        assert_eq!(
            (
                hourly.at(1.0, 60.0),
                hourly.at(3.0, 60.0),
                hourly.at(21.0, 60.0)
            ),
            (10.0, 1.0, 10.0)
        );
    }

    #[test]
    fn chaos_and_curves_read_from_toml_style_json() {
        let c: Curve =
            serde_json::from_str(r#"{ "spike": { "at": 20, "x": 4, "for": 15 } }"#).unwrap();
        assert_eq!(
            c,
            Curve::Spike {
                at: 20.0,
                x: 4.0,
                for_s: 15.0
            }
        );
        let k: ChaosEvent =
            serde_json::from_str(r#"{ "kill": { "kind": "sql", "role": "primary", "at": 30 } }"#)
                .unwrap();
        assert_eq!(
            k,
            ChaosEvent::Kill {
                target: Selector {
                    kind: Some(ComponentKind::Sql),
                    role: Some(Role::Primary),
                    ..Default::default()
                },
                at: 30.0,
                for_s: None
            }
        );
        assert_eq!(k.at(), 30.0);
    }

    #[test]
    fn pass_criteria_merge() {
        let base = PassCriteria {
            p95_ms: Some(80.0),
            error_rate: Some(0.001),
            ..Default::default()
        };
        let over = PassCriteria {
            p95_ms: Some(120.0),
            ..Default::default()
        };
        let m = base.merged(&over);
        assert_eq!((m.p95_ms, m.error_rate), (Some(120.0), Some(0.001)));
    }
}
