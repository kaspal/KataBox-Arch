//! The component registry: every kind the canvas offers, defined once, as data
//! (spec → Component library). The UI receives it through the `registry` command and never
//! duplicates it. Display names are not here: the UI translates
//! `registry.kind.<kind>`, `registry.flavour.<flavour>`, `registry.prop.<key>`,
//! `registry.choice.<option>` and `units.<unit>`.
//!
//! Performance defaults (instances, service times, hit rates, …) are placeholders in M0.
//! M1 calibrates each flavour's profile and cites the vendor's benchmarks or docs here.

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
    }
}

fn float(key: &str, min: f64, max: f64, step: f64, default: f64, unit: Unit) -> PropSpec {
    PropSpec {
        key: key.into(),
        ty: PropType::Float { min, max, step },
        default: PropValue::Number(default),
        unit,
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
    }
}

fn boolean(key: &str, default: bool) -> PropSpec {
    PropSpec {
        key: key.into(),
        ty: PropType::Bool,
        default: PropValue::Bool(default),
        unit: Unit::None,
    }
}

fn fl(flavour: Flavour) -> FlavourSpec {
    FlavourSpec {
        flavour,
        overrides: Vec::new(),
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
    }
}

fn num(v: f64) -> PropValue {
    PropValue::Number(v)
}

fn kind(kind: ComponentKind, flavours: Vec<FlavourSpec>, props: Vec<PropSpec>) -> KindSpec {
    let category = kind.category();
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

// Shared prop sets. M1: calibrate + cite every default.
fn instances(default: f64) -> PropSpec {
    int("instances", 1.0, 64.0, default, Unit::Count)
}
fn concurrency(default: f64) -> PropSpec {
    int("concurrency", 1.0, 4096.0, default, Unit::Count)
}
fn service_ms(default: f64) -> PropSpec {
    float("service_ms", 0.1, 10_000.0, 0.1, default, Unit::Ms)
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
                    &["round_robin", "least_conn", "ip_hash"],
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
                fl_with(F::Django, &[("service_ms", num(20.0))]),
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
            vec![fl(F::Celery), fl(F::Sidekiq)],
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
                fl_with(F::CloudRun, &[("cold_start_ms", num(500.0))]),
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
            vec![fl(F::Postgresql), fl(F::Mysql)],
            vec![
                int("shards", 1.0, 64.0, 1.0, Unit::Count),
                int("concurrency", 1.0, 1000.0, 100.0, Unit::Count),
                service_ms(5.0),
                float("replication_lag_ms", 0.0, 10_000.0, 1.0, 50.0, Unit::Ms),
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
                service_ms(0.5),
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
