//! The component registry: every kind the canvas offers, defined once, as data
//! (spec → Component library). The UI receives it through the `registry` command and never
//! duplicates it. Display names are not here: the UI translates
//! `registry.kind.<kind>`, `registry.flavour.<flavour>`, `registry.prop.<key>`,
//! `registry.choice.<option>` and `units.<unit>`.
//!
//! Behaviour figures (service times, concurrency, cold starts, replication lag, queue sizes,
//! prices) are calibrated per flavour (ADR 0005): every one has a row in
//! `docs/calibration.md` with its value, unit and source, and a test keeps the two in step.
//! Props that are design choices (instances, shards, memory, TTLs…) are starting points,
//! not calibrated figures.

use serde::{Deserialize, Serialize};

use crate::design::PropValue;
use crate::wiring::{self, WiringTable};

/// The 25 component kinds. Ids match the icon ids (`icons.json`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ComponentKind {
    // Clients & Edge
    Client,
    Dns,
    Cdn,
    Waf,
    Lb,
    Gateway,
    Ratelimit,
    // Compute
    App,
    Micro,
    Worker,
    Serverless,
    Autoscale,
    // Data
    Sql,
    Nosql,
    Cache,
    Object,
    Search,
    // Async
    Queue,
    Stream,
    Pubsub,
    Dlq,
    Scheduler,
    // Ops
    Metrics,
    Logs,
    Tracing,
}

impl ComponentKind {
    pub const ALL: [ComponentKind; 25] = [
        ComponentKind::Client,
        ComponentKind::Dns,
        ComponentKind::Cdn,
        ComponentKind::Waf,
        ComponentKind::Lb,
        ComponentKind::Gateway,
        ComponentKind::Ratelimit,
        ComponentKind::App,
        ComponentKind::Micro,
        ComponentKind::Worker,
        ComponentKind::Serverless,
        ComponentKind::Autoscale,
        ComponentKind::Sql,
        ComponentKind::Nosql,
        ComponentKind::Cache,
        ComponentKind::Object,
        ComponentKind::Search,
        ComponentKind::Queue,
        ComponentKind::Stream,
        ComponentKind::Pubsub,
        ComponentKind::Dlq,
        ComponentKind::Scheduler,
        ComponentKind::Metrics,
        ComponentKind::Logs,
        ComponentKind::Tracing,
    ];

    /// The snake_case id (the serde name, the icon id and the i18n key).
    pub fn id(self) -> &'static str {
        match self {
            ComponentKind::Client => "client",
            ComponentKind::Dns => "dns",
            ComponentKind::Cdn => "cdn",
            ComponentKind::Waf => "waf",
            ComponentKind::Lb => "lb",
            ComponentKind::Gateway => "gateway",
            ComponentKind::Ratelimit => "ratelimit",
            ComponentKind::App => "app",
            ComponentKind::Micro => "micro",
            ComponentKind::Worker => "worker",
            ComponentKind::Serverless => "serverless",
            ComponentKind::Autoscale => "autoscale",
            ComponentKind::Sql => "sql",
            ComponentKind::Nosql => "nosql",
            ComponentKind::Cache => "cache",
            ComponentKind::Object => "object",
            ComponentKind::Search => "search",
            ComponentKind::Queue => "queue",
            ComponentKind::Stream => "stream",
            ComponentKind::Pubsub => "pubsub",
            ComponentKind::Dlq => "dlq",
            ComponentKind::Scheduler => "scheduler",
            ComponentKind::Metrics => "metrics",
            ComponentKind::Logs => "logs",
            ComponentKind::Tracing => "tracing",
        }
    }

    pub fn category(self) -> Category {
        use ComponentKind::*;
        match self {
            Client | Dns | Cdn | Waf | Lb | Gateway | Ratelimit => Category::Edge,
            App | Micro | Worker | Serverless | Autoscale => Category::Compute,
            Sql | Nosql | Cache | Object | Search => Category::Data,
            Queue | Stream | Pubsub | Dlq | Scheduler => Category::Async,
            Metrics | Logs | Tracing => Category::Ops,
        }
    }
}

/// The five library categories, each with its colour token (`--cat-<id>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Clients & Edge (blue)
    Edge,
    /// Compute (green)
    Compute,
    /// Data (terracotta)
    Data,
    /// Async (amber)
    Async,
    /// Ops (ink)
    Ops,
}

impl Category {
    pub const ALL: [Category; 5] = [
        Category::Edge,
        Category::Compute,
        Category::Data,
        Category::Async,
        Category::Ops,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Category::Edge => "edge",
            Category::Compute => "compute",
            Category::Data => "data",
            Category::Async => "async",
            Category::Ops => "ops",
        }
    }
}

/// Real-product flavours. A flavour belongs to one kind (`Generic` is shared by kinds with
/// no product choice yet). Vendor names belong to their owners; no logos are used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Flavour {
    Generic,
    // client
    Web,
    Mobile,
    // dns
    // Explicit: specta and serde split the digits differently (`route_53` vs `route53`).
    #[serde(rename = "route53")]
    Route53,
    CloudflareDns,
    // cdn
    Cloudfront,
    CloudflareCdn,
    Fastly,
    // waf
    AwsWaf,
    CloudflareWaf,
    // lb
    Nginx,
    Haproxy,
    AwsAlb,
    AwsNlb,
    // gateway
    Kong,
    AwsApiGateway,
    // ratelimit
    Envoy,
    RedisTokenBucket,
    // app
    Nodejs,
    SpringBoot,
    Go,
    Django,
    // worker
    Celery,
    Sidekiq,
    // serverless
    AwsLambda,
    CloudRun,
    // autoscale
    KubernetesHpa,
    AwsAsg,
    // sql
    Postgresql,
    Mysql,
    // nosql
    Dynamodb,
    Cassandra,
    Mongodb,
    // cache
    Redis,
    Memcached,
    // object
    S3,
    Gcs,
    // search
    Elasticsearch,
    Opensearch,
    // queue
    Rabbitmq,
    Sqs,
    // stream
    Kafka,
    Kinesis,
    // pubsub
    GooglePubsub,
    Sns,
    RedisPubsub,
    // scheduler
    Cron,
    Airflow,
    // ops
    PrometheusGrafana,
    Elk,
    Loki,
    Jaeger,
}

/// The unit shown after a prop's value (`units.<unit>` in the UI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    None,
    Count,
    Ms,
    S,
    H,
    Days,
    Percent,
    Rps,
    Gb,
}

/// A prop's type and allowed range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PropType {
    /// A whole number in `[min, max]`.
    Int {
        min: f64,
        max: f64,
        step: f64,
    },
    Float {
        min: f64,
        max: f64,
        step: f64,
    },
    Bool,
    /// One of `options` (ids, translated as `registry.choice.<option>`).
    Choice {
        options: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PropSpec {
    pub key: String,
    pub ty: PropType,
    pub default: PropValue,
    pub unit: Unit,
    /// Set when the default is a calibrated figure (a row in `docs/calibration.md`); None
    /// for design choices.
    #[serde(default)]
    pub calibration: Option<CalibrationRef>,
}

/// Where a calibrated default comes from (ADR 0005).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct CalibrationRef {
    /// The row id in `docs/calibration.md`, e.g. `sql.service_ms`.
    pub id: String,
    pub status: CalibrationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum CalibrationStatus {
    /// The figure is taken from the cited source.
    Cited,
    /// A conservative ballpark; its source still has to be confirmed.
    Needed,
}

/// Props whose defaults are calibrated figures rather than design choices.
pub const CALIBRATED_PROPS: [&str; 8] = [
    "service_ms",
    "concurrency",
    "concurrency_limit",
    "cold_start_ms",
    "replication_lag_ms",
    "failover_s",
    "visibility_timeout_s",
    "session_timeout_s",
];

/// Calibration row ids whose source has been confirmed (the planning chat checked these on
/// 2026-11-22; `docs/calibration.md` quotes each source). Every other row is `source: needed`.
pub const CITED: [&str; 13] = [
    "sql.concurrency",
    "sql.concurrency@mysql",
    "sql.failover_s",
    "cache.service_ms",
    "nosql.servers_per_instance@dynamodb",
    "object.servers_per_instance",
    "serverless.concurrency_limit",
    "serverless.concurrency_limit@cloud_run",
    "serverless.cold_start_ms",
    "queue.visibility_timeout_s",
    "search.servers_per_instance",
    "stream.session_timeout_s",
    "nosql.write_slots@dynamodb",
];

fn calibration_status(id: &str) -> CalibrationStatus {
    if CITED.contains(&id) {
        CalibrationStatus::Cited
    } else {
        CalibrationStatus::Needed
    }
}

/// How a flavour behaves as a server where it has no prop for it (ADR 0005, ADR 0006).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct SimDefaults {
    /// Requests one instance (or one partition/shard) works on at once, when the kind has
    /// no `concurrency` prop.
    pub servers_per_instance: f64,
    /// Median service time when the kind has no `service_ms` prop.
    pub service_ms: f64,
    /// p99 / median of the service time.
    pub p99_ratio: f64,
    /// Waiting slots per server before requests are dropped.
    pub queue_per_server: f64,
    /// Seconds before a replica takes over from a dead primary (data kinds).
    pub failover_s: f64,
    /// Service slots a write occupies, relative to a read (data kinds; 1 = the same).
    /// DynamoDB partitions take a third as many writes as reads per second (ADR 0006).
    pub write_slots: f64,
}

impl PropSpec {
    /// Is `value` acceptable for this prop?
    pub fn accepts(&self, value: &PropValue) -> bool {
        match (&self.ty, value) {
            (PropType::Int { min, max, .. }, PropValue::Number(n)) => {
                n.is_finite() && n.fract() == 0.0 && *n >= *min && *n <= *max
            }
            (PropType::Float { min, max, .. }, PropValue::Number(n)) => {
                n.is_finite() && *n >= *min && *n <= *max
            }
            (PropType::Bool, PropValue::Bool(_)) => true,
            (PropType::Choice { options }, PropValue::Text(s)) => options.iter().any(|o| o == s),
            _ => false,
        }
    }

    /// Does `value` have the right type (in or out of range)?
    pub fn type_matches(&self, value: &PropValue) -> bool {
        matches!(
            (&self.ty, value),
            (
                PropType::Int { .. } | PropType::Float { .. },
                PropValue::Number(_)
            ) | (PropType::Bool, PropValue::Bool(_))
                | (PropType::Choice { .. }, PropValue::Text(_))
        )
    }
}

/// A flavour of a kind, with the props it changes from the kind's defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct FlavourSpec {
    pub flavour: Flavour,
    pub overrides: Vec<PropOverride>,
    /// The monthly price model (a teaching model, consistent across flavours; not a quote).
    pub cost: Cost,
    /// Behaviour without a prop; None for kinds never on a request's path.
    pub sim: Option<SimDefaults>,
}

/// A flavour's monthly price: per instance (or per card), per GB of cache memory, and per
/// million requests through it. Teaching figures, loosely shaped like public list prices
/// (ADR 0006); the judge compares designs with each other, never with a real bill.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Cost {
    pub per_instance_month: f64,
    pub per_gb_month: f64,
    pub per_million: f64,
}

/// The kind's behaviour where it has no prop (`docs/calibration.md`, rows `<kind>.<field>`).
fn kind_sim(kind: ComponentKind) -> Option<SimDefaults> {
    use ComponentKind as K;
    let d = |servers_per_instance: f64, service_ms: f64, p99_ratio: f64, queue_per_server: f64| {
        Some(SimDefaults {
            servers_per_instance,
            service_ms,
            p99_ratio,
            queue_per_server,
            failover_s: 0.0,
            write_slots: 1.0,
        })
    };
    let mut s = match kind {
        K::Dns => d(10_000.0, 0.2, 3.0, 4.0),
        K::Cdn => d(100_000.0, 2.0, 4.0, 4.0),
        K::Waf => d(2_000.0, 1.0, 3.0, 8.0),
        K::Lb => d(10_000.0, 0.5, 3.0, 8.0),
        K::Gateway => d(256.0, 2.0, 4.0, 8.0),
        K::Ratelimit => d(10_000.0, 0.3, 3.0, 8.0),
        K::App | K::Micro => d(64.0, 10.0, 4.0, 8.0),
        K::Worker => d(8.0, 100.0, 4.0, 8.0),
        K::Serverless => d(1.0, 20.0, 4.0, 0.0),
        K::Sql => d(100.0, 5.0, 5.0, 4.0),
        // Per partition.
        K::Nosql => d(32.0, 3.0, 4.0, 4.0),
        // One event loop per instance.
        K::Cache => d(1.0, 0.01, 3.0, 512.0),
        K::Object => d(1_000.0, 50.0, 4.0, 4.0),
        // Per shard: the search thread pool on 8 vCPUs, int(3 × 8 / 2) + 1, and its queue.
        K::Search => d(13.0, 20.0, 4.0, 77.0),
        K::Queue => d(10_000.0, 2.0, 3.0, 4.0),
        K::Stream => d(10_000.0, 3.0, 3.0, 4.0),
        K::Pubsub => d(10_000.0, 5.0, 3.0, 4.0),
        K::Dlq => d(10_000.0, 2.0, 3.0, 4.0),
        K::Client | K::Scheduler | K::Autoscale | K::Metrics | K::Logs | K::Tracing => None,
    };
    if let Some(s) = &mut s {
        s.failover_s = match kind {
            K::Sql => 30.0,
            K::Cache => 15.0,
            K::Search => 10.0,
            K::Nosql => 5.0,
            _ => 0.0,
        };
    }
    s
}

/// A flavour's departures from its kind's behaviour.
fn flavour_sim(flavour: Flavour, s: &mut SimDefaults) {
    use Flavour as F;
    match flavour {
        // DynamoDB: about 3,000 reads/s per partition at ~3 ms, and a third as many writes.
        F::Dynamodb => {
            s.servers_per_instance = 8.0;
            s.write_slots = 3.0;
        }
        // Memcached: 4 worker threads by default.
        F::Memcached => s.servers_per_instance = 4.0,
        F::Sqs => s.service_ms = 10.0,
        F::Kinesis => s.service_ms = 15.0,
        _ => {}
    }
}

/// The kind's default price (flavours may override in `fl_cost`).
fn kind_cost(kind: ComponentKind) -> Cost {
    use ComponentKind as K;
    let c = |i: f64, g: f64, m: f64| Cost {
        per_instance_month: i,
        per_gb_month: g,
        per_million: m,
    };
    match kind {
        K::Client | K::Autoscale => c(0.0, 0.0, 0.0),
        K::Dns => c(0.0, 0.0, 0.4),
        K::Cdn => c(0.0, 0.0, 0.75),
        K::Waf => c(20.0, 0.0, 0.6),
        K::Lb => c(25.0, 0.0, 0.0),
        K::Gateway => c(60.0, 0.0, 1.0),
        K::Ratelimit => c(30.0, 0.0, 0.0),
        K::App | K::Micro | K::Worker => c(70.0, 0.0, 0.0),
        K::Serverless => c(0.0, 0.0, 0.5),
        K::Sql => c(350.0, 0.0, 0.0),
        K::Nosql => c(120.0, 0.0, 0.5),
        K::Cache => c(0.0, 18.0, 0.0),
        K::Object => c(0.0, 0.0, 1.0),
        K::Search => c(250.0, 0.0, 0.0),
        K::Queue => c(40.0, 0.0, 0.4),
        K::Stream => c(15.0, 0.0, 0.2),
        K::Pubsub => c(0.0, 0.0, 0.5),
        K::Dlq => c(0.0, 0.0, 0.4),
        K::Scheduler => c(20.0, 0.0, 0.0),
        K::Metrics => c(100.0, 0.0, 0.0),
        K::Logs => c(150.0, 0.0, 0.0),
        K::Tracing => c(80.0, 0.0, 0.0),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct PropOverride {
    pub key: String,
    pub value: PropValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct KindSpec {
    pub kind: ComponentKind,
    pub category: Category,
    /// Glyph id (the UI's icon set; flavours share their kind's glyph).
    pub icon: String,
    /// CSS colour token of the category, e.g. `cat-edge`.
    pub color_token: String,
    pub flavours: Vec<FlavourSpec>,
    pub default_flavour: Flavour,
    pub props: Vec<PropSpec>,
    /// How the kind sits on the canvas: a card, or a wrapper around cards (Autoscaling).
    pub placement: Placement,
}

/// How a kind is placed on the canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    /// An index card wired with edges.
    Card,
    /// A resizable frame around compute cards, holding a policy (an autoscaling group).
    Wrapper,
}

impl KindSpec {
    pub fn flavour(&self, flavour: Flavour) -> Option<&FlavourSpec> {
        self.flavours.iter().find(|f| f.flavour == flavour)
    }

    pub fn prop(&self, key: &str) -> Option<&PropSpec> {
        self.props.iter().find(|p| p.key == key)
    }
}

/// Canvas items that are not components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum MetaKind {
    Region,
    Az,
    Note,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct MetaSpec {
    pub kind: MetaKind,
    pub icon: String,
}

/// Everything the editor needs to know about components, sent to the UI once.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Registry {
    pub categories: Vec<Category>,
    pub kinds: Vec<KindSpec>,
    pub meta: Vec<MetaSpec>,
    pub wiring: WiringTable,
    pub limits: crate::validate::Limits,
}

impl Registry {
    pub fn kind(&self, kind: ComponentKind) -> &KindSpec {
        self.kinds
            .iter()
            .find(|k| k.kind == kind)
            .expect("every ComponentKind is in the registry")
    }
}

// --- Builders (keep the table below readable) -----------------------------------------

fn int(key: &str, min: f64, max: f64, default: f64, unit: Unit) -> PropSpec {
    PropSpec {
        key: key.into(),
        ty: PropType::Int {
            min,
            max,
            step: 1.0,
        },
        default: PropValue::Number(default),
        unit,
        calibration: None,
    }
}

fn float(key: &str, min: f64, max: f64, step: f64, default: f64, unit: Unit) -> PropSpec {
    PropSpec {
        key: key.into(),
        ty: PropType::Float { min, max, step },
        default: PropValue::Number(default),
        unit,
        calibration: None,
    }
}

fn choice(key: &str, options: &[&str], default: &str) -> PropSpec {
    PropSpec {
        key: key.into(),
        ty: PropType::Choice {
            options: options.iter().map(|o| (*o).into()).collect(),
        },
        default: PropValue::Text(default.into()),
        unit: Unit::None,
        calibration: None,
    }
}

fn boolean(key: &str, default: bool) -> PropSpec {
    PropSpec {
        key: key.into(),
        ty: PropType::Bool,
        default: PropValue::Bool(default),
        unit: Unit::None,
        calibration: None,
    }
}

fn fl(flavour: Flavour) -> FlavourSpec {
    FlavourSpec {
        flavour,
        overrides: Vec::new(),
        cost: Cost::default(),
        sim: None,
    }
}

fn fl_with(flavour: Flavour, overrides: &[(&str, PropValue)]) -> FlavourSpec {
    FlavourSpec {
        flavour,
        overrides: overrides
            .iter()
            .map(|(k, v)| PropOverride {
                key: (*k).into(),
                value: v.clone(),
            })
            .collect(),
        cost: Cost::default(),
        sim: None,
    }
}

fn num(v: f64) -> PropValue {
    PropValue::Number(v)
}

fn kind(kind: ComponentKind, mut flavours: Vec<FlavourSpec>, mut props: Vec<PropSpec>) -> KindSpec {
    let category = kind.category();
    for f in &mut flavours {
        if f.cost == Cost::default() {
            f.cost = kind_cost(kind);
        }
        f.sim = kind_sim(kind).map(|mut s| {
            flavour_sim(f.flavour, &mut s);
            s
        });
    }
    for p in &mut props {
        if CALIBRATED_PROPS.contains(&p.key.as_str()) {
            let id = format!("{}.{}", kind.id(), p.key);
            p.calibration = Some(CalibrationRef {
                status: calibration_status(&id),
                id,
            });
        }
    }
    KindSpec {
        kind,
        category,
        icon: kind.id().into(),
        color_token: format!("cat-{}", category.id()),
        default_flavour: flavours[0].flavour,
        flavours,
        props,
        placement: if kind == ComponentKind::Autoscale {
            Placement::Wrapper
        } else {
            Placement::Card
        },
    }
}

// Shared prop sets.
fn instances(default: f64) -> PropSpec {
    int("instances", 1.0, 64.0, default, Unit::Count)
}
fn concurrency(default: f64) -> PropSpec {
    int("concurrency", 1.0, 4096.0, default, Unit::Count)
}
fn service_ms(default: f64) -> PropSpec {
    float("service_ms", 0.01, 10_000.0, 0.01, default, Unit::Ms)
}
fn compute_props(inst: f64, conc: f64, ms: f64) -> Vec<PropSpec> {
    vec![instances(inst), concurrency(conc), service_ms(ms)]
}

/// The registry. Order is the library's order: categories, then kinds within each.
pub fn registry() -> Registry {
    use ComponentKind as K;
    use Flavour as F;
    let kinds = vec![
        // --- Clients & Edge ---
        kind(
            K::Client,
            vec![fl(F::Web), fl(F::Mobile)],
            vec![int("rps", 1.0, 1_000_000.0, 1000.0, Unit::Rps)],
        ),
        kind(
            K::Dns,
            vec![fl(F::Route53), fl(F::CloudflareDns)],
            vec![int("ttl_s", 0.0, 86_400.0, 300.0, Unit::S)],
        ),
        kind(
            K::Cdn,
            vec![fl(F::Cloudfront), fl(F::CloudflareCdn), fl(F::Fastly)],
            vec![int("ttl_s", 0.0, 604_800.0, 3600.0, Unit::S)],
        ),
        kind(
            K::Waf,
            vec![fl(F::AwsWaf), fl(F::CloudflareWaf)],
            vec![instances(1.0), service_ms(1.0)],
        ),
        kind(
            K::Lb,
            vec![
                fl(F::Nginx),
                fl(F::Haproxy),
                fl(F::AwsAlb),
                fl_with(F::AwsNlb, &[("service_ms", num(0.2))]),
            ],
            vec![
                int("instances", 1.0, 16.0, 1.0, Unit::Count),
                choice(
                    "algorithm",
                    &["round_robin", "least_conn", "key_hash"],
                    "round_robin",
                ),
                service_ms(0.5),
            ],
        ),
        kind(
            K::Gateway,
            vec![fl(F::Kong), fl(F::AwsApiGateway)],
            compute_props(2.0, 256.0, 2.0),
        ),
        kind(
            K::Ratelimit,
            vec![fl(F::Envoy), fl(F::RedisTokenBucket)],
            vec![
                int("limit_rps", 1.0, 1_000_000.0, 1000.0, Unit::Rps),
                int("burst", 1.0, 100_000.0, 100.0, Unit::Count),
            ],
        ),
        // --- Compute ---
        kind(
            K::App,
            vec![
                fl(F::Nodejs),
                fl(F::SpringBoot),
                fl_with(F::Go, &[("service_ms", num(5.0))]),
                fl_with(
                    F::Django,
                    &[("service_ms", num(20.0)), ("concurrency", num(9.0))],
                ),
            ],
            compute_props(2.0, 64.0, 10.0),
        ),
        kind(
            K::Micro,
            vec![fl(F::Generic)],
            compute_props(2.0, 64.0, 10.0),
        ),
        kind(
            K::Worker,
            vec![
                fl(F::Celery),
                fl_with(F::Sidekiq, &[("concurrency", num(5.0))]),
            ],
            vec![
                instances(2.0),
                int("concurrency", 1.0, 1024.0, 8.0, Unit::Count),
                service_ms(100.0),
            ],
        ),
        kind(
            K::Serverless,
            vec![
                fl(F::AwsLambda),
                fl_with(
                    F::CloudRun,
                    &[
                        ("cold_start_ms", num(500.0)),
                        ("concurrency_limit", num(8000.0)),
                    ],
                ),
            ],
            vec![
                int("concurrency_limit", 1.0, 10_000.0, 1000.0, Unit::Count),
                float("cold_start_ms", 0.0, 10_000.0, 1.0, 250.0, Unit::Ms),
                service_ms(20.0),
            ],
        ),
        kind(
            K::Autoscale,
            vec![fl(F::KubernetesHpa), fl(F::AwsAsg)],
            vec![
                int("min_instances", 1.0, 64.0, 2.0, Unit::Count),
                int("max_instances", 1.0, 256.0, 10.0, Unit::Count),
                int("target_util", 10.0, 95.0, 60.0, Unit::Percent),
                int("scale_delay_s", 0.0, 600.0, 60.0, Unit::S),
            ],
        ),
        // --- Data ---
        kind(
            K::Sql,
            vec![
                fl(F::Postgresql),
                fl_with(F::Mysql, &[("concurrency", num(151.0))]),
            ],
            vec![
                int("shards", 1.0, 64.0, 1.0, Unit::Count),
                int("concurrency", 1.0, 1000.0, 100.0, Unit::Count),
                service_ms(5.0),
                float("replication_lag_ms", 0.0, 10_000.0, 1.0, 50.0, Unit::Ms),
                int("failover_s", 1.0, 600.0, 30.0, Unit::S),
                choice("read_from", &["any", "primary"], "any"),
                int("cdc_buffer_s", 0.0, 3600.0, 300.0, Unit::S),
            ],
        ),
        kind(
            K::Nosql,
            vec![fl(F::Dynamodb), fl(F::Cassandra), fl(F::Mongodb)],
            vec![
                int("partitions", 1.0, 1024.0, 4.0, Unit::Count),
                int("replication_factor", 1.0, 5.0, 3.0, Unit::Count),
                choice("consistency", &["eventual", "strong"], "eventual"),
                service_ms(3.0),
                int("cdc_buffer_s", 0.0, 3600.0, 300.0, Unit::S),
            ],
        ),
        kind(
            K::Cache,
            vec![fl(F::Redis), fl(F::Memcached)],
            vec![
                float("memory_gb", 0.5, 1024.0, 0.5, 2.0, Unit::Gb),
                int("ttl_s", 0.0, 604_800.0, 300.0, Unit::S),
                choice("eviction", &["lru", "lfu", "ttl"], "lru"),
                boolean("persistence", false),
                service_ms(0.01),
                int("failover_s", 1.0, 600.0, 15.0, Unit::S),
            ],
        ),
        kind(
            K::Object,
            vec![fl(F::S3), fl(F::Gcs)],
            vec![service_ms(50.0)],
        ),
        kind(
            K::Search,
            vec![fl(F::Elasticsearch), fl(F::Opensearch)],
            vec![
                int("shards", 1.0, 64.0, 2.0, Unit::Count),
                int("replicas", 0.0, 5.0, 1.0, Unit::Count),
                service_ms(20.0),
            ],
        ),
        // --- Async ---
        kind(
            K::Queue,
            vec![fl(F::Rabbitmq), fl(F::Sqs)],
            vec![
                int("max_depth", 1.0, 10_000_000.0, 100_000.0, Unit::Count),
                int("visibility_timeout_s", 1.0, 43_200.0, 30.0, Unit::S),
                int("max_retries", 0.0, 20.0, 3.0, Unit::Count),
            ],
        ),
        kind(
            K::Stream,
            vec![fl(F::Kafka), fl(F::Kinesis)],
            vec![
                int("partitions", 1.0, 1024.0, 6.0, Unit::Count),
                int("session_timeout_s", 1.0, 1800.0, 45.0, Unit::S),
                int("retention_h", 1.0, 8760.0, 168.0, Unit::H),
            ],
        ),
        kind(
            K::Pubsub,
            vec![fl(F::GooglePubsub), fl(F::Sns), fl(F::RedisPubsub)],
            vec![service_ms(5.0)],
        ),
        kind(
            K::Dlq,
            vec![fl(F::Generic)],
            vec![int("retention_days", 1.0, 14.0, 4.0, Unit::Days)],
        ),
        kind(
            K::Scheduler,
            vec![fl(F::Cron), fl(F::Airflow)],
            vec![int("interval_s", 1.0, 86_400.0, 60.0, Unit::S)],
        ),
        // --- Ops ---
        kind(
            K::Metrics,
            vec![fl(F::PrometheusGrafana)],
            vec![int("retention_days", 1.0, 365.0, 15.0, Unit::Days)],
        ),
        kind(
            K::Logs,
            vec![fl(F::Elk), fl(F::Loki)],
            vec![int("retention_days", 1.0, 365.0, 7.0, Unit::Days)],
        ),
        kind(
            K::Tracing,
            vec![fl(F::Jaeger)],
            vec![float("sample_rate", 0.0, 100.0, 1.0, 10.0, Unit::Percent)],
        ),
    ];
    Registry {
        categories: Category::ALL.to_vec(),
        kinds,
        meta: vec![
            MetaSpec {
                kind: MetaKind::Region,
                icon: "zone".into(),
            },
            MetaSpec {
                kind: MetaKind::Az,
                icon: "zone".into(),
            },
            MetaSpec {
                kind: MetaKind::Note,
                icon: "note".into(),
            },
        ],
        wiring: wiring::table(),
        limits: crate::validate::Limits::default(),
    }
}

/// A flavour's id, as files and the UI spell it.
pub fn flavour_id(f: Flavour) -> String {
    match serde_json::to_value(f) {
        Ok(serde_json::Value::String(s)) => s,
        _ => unreachable!("flavours serialize as strings"),
    }
}

/// One calibrated figure: its row id in `docs/calibration.md` and its value.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationRow {
    pub id: String,
    pub value: f64,
}

/// Every calibrated figure the registry holds, in library order:
/// - calibrated props: `<kind>.<key>`, and `<kind>.<key>@<flavour>` for a flavour override;
/// - behaviour without a prop: `<kind>.<field>` (`servers_per_instance`, `service_ms` when the
///   kind has no such prop, `p99_ratio`, `queue_per_server`, `failover_s` when set), and
///   `@<flavour>` where a flavour departs from its kind;
/// - prices: `<kind>.cost.instance|gb|million` when non-zero, `@<flavour>` likewise.
pub fn calibration_rows(registry: &Registry) -> Vec<CalibrationRow> {
    let mut rows = Vec::new();
    let mut push = |id: String, value: f64| rows.push(CalibrationRow { id, value });
    for k in &registry.kinds {
        let kid = k.kind.id();
        for p in k.props.iter().filter(|p| p.calibration.is_some()) {
            if let PropValue::Number(v) = p.default {
                push(format!("{kid}.{}", p.key), v);
            }
            for f in &k.flavours {
                for o in f.overrides.iter().filter(|o| o.key == p.key) {
                    if let PropValue::Number(v) = o.value {
                        push(format!("{kid}.{}@{}", p.key, flavour_id(f.flavour)), v);
                    }
                }
            }
        }
        let base = kind_sim(k.kind);
        if let Some(b) = base {
            let fields = |s: &SimDefaults| {
                let mut v = Vec::new();
                if k.prop("concurrency").is_none() && k.prop("concurrency_limit").is_none() {
                    v.push(("servers_per_instance", s.servers_per_instance));
                }
                if k.prop("service_ms").is_none() {
                    v.push(("service_ms", s.service_ms));
                }
                v.push(("p99_ratio", s.p99_ratio));
                v.push(("queue_per_server", s.queue_per_server));
                if s.failover_s > 0.0 && k.prop("failover_s").is_none() {
                    v.push(("failover_s", s.failover_s));
                }
                if k.kind == ComponentKind::Nosql {
                    v.push(("write_slots", s.write_slots));
                }
                v
            };
            for (name, v) in fields(&b) {
                push(format!("{kid}.{name}"), v);
            }
            for f in &k.flavours {
                let Some(fs) = f.sim else { continue };
                for ((name, v), (_, bv)) in fields(&fs).into_iter().zip(fields(&b)) {
                    if v != bv {
                        push(format!("{kid}.{name}@{}", flavour_id(f.flavour)), v);
                    }
                }
            }
        }
        let cost_fields = |c: &Cost| {
            [
                ("instance", c.per_instance_month),
                ("gb", c.per_gb_month),
                ("million", c.per_million),
            ]
        };
        let kc = kind_cost(k.kind);
        for (name, v) in cost_fields(&kc) {
            if v != 0.0 {
                push(format!("{kid}.cost.{name}"), v);
            }
        }
        for f in &k.flavours {
            for ((name, v), (_, bv)) in cost_fields(&f.cost).into_iter().zip(cost_fields(&kc)) {
                if v != bv {
                    push(format!("{kid}.cost.{name}@{}", flavour_id(f.flavour)), v);
                }
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn has_25_kinds_in_5_categories_in_library_order() {
        let r = registry();
        assert_eq!(r.kinds.len(), 25);
        let kinds: Vec<_> = r.kinds.iter().map(|k| k.kind).collect();
        assert_eq!(kinds, ComponentKind::ALL.to_vec());
        let per_category: Vec<usize> = Category::ALL
            .iter()
            .map(|c| r.kinds.iter().filter(|k| k.category == *c).count())
            .collect();
        assert_eq!(per_category, vec![7, 5, 5, 5, 3]);
    }

    #[test]
    fn ids_icons_and_tokens_are_consistent() {
        let r = registry();
        let icons: HashSet<_> = r.kinds.iter().map(|k| k.icon.as_str()).collect();
        assert_eq!(icons.len(), 25, "every kind has its own glyph");
        for k in &r.kinds {
            assert_eq!(k.icon, k.kind.id());
            assert_eq!(k.color_token, format!("cat-{}", k.category.id()));
            // The serde name is the id the UI and the files use.
            assert_eq!(serde_json::to_value(k.kind).unwrap(), k.kind.id());
        }
    }

    #[test]
    fn flavours_belong_to_one_kind_except_generic() {
        let r = registry();
        let mut owner = std::collections::HashMap::new();
        for k in &r.kinds {
            assert!(!k.flavours.is_empty(), "{:?} has a flavour", k.kind);
            assert!(
                k.flavour(k.default_flavour).is_some(),
                "{:?} default is listed",
                k.kind
            );
            for f in &k.flavours {
                if f.flavour == Flavour::Generic {
                    continue;
                }
                if let Some(prev) = owner.insert(f.flavour, k.kind) {
                    panic!("{:?} is listed under {prev:?} and {:?}", f.flavour, k.kind);
                }
            }
        }
    }

    #[test]
    fn defaults_and_overrides_are_in_range() {
        let r = registry();
        for k in &r.kinds {
            let mut keys = HashSet::new();
            for p in &k.props {
                assert!(
                    keys.insert(&p.key),
                    "{:?}.{} is defined twice",
                    k.kind,
                    p.key
                );
                assert!(
                    p.accepts(&p.default),
                    "{:?}.{} default out of range",
                    k.kind,
                    p.key
                );
                if let PropType::Int { min, max, step } | PropType::Float { min, max, step } = p.ty
                {
                    assert!(min < max && step > 0.0, "{:?}.{} range", k.kind, p.key);
                }
            }
            for f in &k.flavours {
                for o in &f.overrides {
                    let spec = k.prop(&o.key).unwrap_or_else(|| {
                        panic!("{:?}/{:?} overrides unknown {}", k.kind, f.flavour, o.key)
                    });
                    assert!(
                        spec.accepts(&o.value),
                        "{:?}/{:?}.{}",
                        k.kind,
                        f.flavour,
                        o.key
                    );
                }
            }
        }
    }

    #[test]
    fn int_props_take_whole_numbers_only() {
        let p = int("n", 0.0, 10.0, 1.0, Unit::Count);
        assert!(p.accepts(&PropValue::Number(2.0)));
        assert!(!p.accepts(&PropValue::Number(2.5)));
        assert!(!p.accepts(&PropValue::Number(11.0)));
        assert!(!p.accepts(&PropValue::Text("2".into())));
        let f = float("x", 0.0, 10.0, 0.5, 1.0, Unit::Ms);
        // A float prop edited to a whole number in the UI arrives as 2: still valid.
        assert!(f.accepts(&PropValue::Number(2.0)));
        assert!(f.accepts(&PropValue::Number(2.5)));
    }
}
