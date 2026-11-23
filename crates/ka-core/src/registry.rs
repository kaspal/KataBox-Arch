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

/// The 42 component kinds. Ids match the glyph ids (`solve/glyphs.ts`).
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
    Ws,
    Idp,
    External,
    // Compute
    App,
    Micro,
    Worker,
    Serverless,
    Autoscale,
    Infer,
    Batch,
    Media,
    // Data
    Sql,
    Nosql,
    Cache,
    Object,
    Search,
    Coord,
    Vector,
    Warehouse,
    Lakehouse,
    Tsdb,
    Dbproxy,
    Graph,
    // Async
    Queue,
    Stream,
    Pubsub,
    Dlq,
    Scheduler,
    Streamproc,
    Mqtt,
    Connector,
    // Ops
    Metrics,
    Logs,
    Tracing,
    Config,
}

impl ComponentKind {
    pub const ALL: [ComponentKind; 42] = [
        ComponentKind::Client,
        ComponentKind::Dns,
        ComponentKind::Cdn,
        ComponentKind::Waf,
        ComponentKind::Lb,
        ComponentKind::Gateway,
        ComponentKind::Ratelimit,
        ComponentKind::Ws,
        ComponentKind::Idp,
        ComponentKind::External,
        ComponentKind::App,
        ComponentKind::Micro,
        ComponentKind::Worker,
        ComponentKind::Serverless,
        ComponentKind::Autoscale,
        ComponentKind::Infer,
        ComponentKind::Batch,
        ComponentKind::Media,
        ComponentKind::Sql,
        ComponentKind::Nosql,
        ComponentKind::Cache,
        ComponentKind::Object,
        ComponentKind::Search,
        ComponentKind::Coord,
        ComponentKind::Vector,
        ComponentKind::Warehouse,
        ComponentKind::Lakehouse,
        ComponentKind::Tsdb,
        ComponentKind::Dbproxy,
        ComponentKind::Graph,
        ComponentKind::Queue,
        ComponentKind::Stream,
        ComponentKind::Pubsub,
        ComponentKind::Dlq,
        ComponentKind::Scheduler,
        ComponentKind::Streamproc,
        ComponentKind::Mqtt,
        ComponentKind::Connector,
        ComponentKind::Metrics,
        ComponentKind::Logs,
        ComponentKind::Tracing,
        ComponentKind::Config,
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
            ComponentKind::Ws => "ws",
            ComponentKind::Idp => "idp",
            ComponentKind::External => "external",
            ComponentKind::App => "app",
            ComponentKind::Micro => "micro",
            ComponentKind::Worker => "worker",
            ComponentKind::Serverless => "serverless",
            ComponentKind::Autoscale => "autoscale",
            ComponentKind::Infer => "infer",
            ComponentKind::Batch => "batch",
            ComponentKind::Media => "media",
            ComponentKind::Sql => "sql",
            ComponentKind::Nosql => "nosql",
            ComponentKind::Cache => "cache",
            ComponentKind::Object => "object",
            ComponentKind::Search => "search",
            ComponentKind::Coord => "coord",
            ComponentKind::Vector => "vector",
            ComponentKind::Warehouse => "warehouse",
            ComponentKind::Lakehouse => "lakehouse",
            ComponentKind::Tsdb => "tsdb",
            ComponentKind::Dbproxy => "dbproxy",
            ComponentKind::Graph => "graph",
            ComponentKind::Queue => "queue",
            ComponentKind::Stream => "stream",
            ComponentKind::Pubsub => "pubsub",
            ComponentKind::Dlq => "dlq",
            ComponentKind::Scheduler => "scheduler",
            ComponentKind::Streamproc => "streamproc",
            ComponentKind::Mqtt => "mqtt",
            ComponentKind::Connector => "connector",
            ComponentKind::Metrics => "metrics",
            ComponentKind::Logs => "logs",
            ComponentKind::Tracing => "tracing",
            ComponentKind::Config => "config",
        }
    }

    pub fn category(self) -> Category {
        use ComponentKind::*;
        match self {
            Client | Dns | Cdn | Waf | Lb | Gateway | Ratelimit | Ws | Idp | External => {
                Category::Edge
            }
            App | Micro | Worker | Serverless | Autoscale | Infer | Batch | Media => {
                Category::Compute
            }
            Sql | Nosql | Cache | Object | Search | Coord | Vector | Warehouse | Lakehouse
            | Tsdb | Dbproxy | Graph => Category::Data,
            Queue | Stream | Pubsub | Dlq | Scheduler | Streamproc | Mqtt | Connector => {
                Category::Async
            }
            Metrics | Logs | Tracing | Config => Category::Ops,
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
    // ws (M8)
    SocketIo,
    AwsApigwWs,
    Phoenix,
    // idp
    #[serde(rename = "auth0")]
    Auth0,
    Cognito,
    Keycloak,
    Okta,
    // external
    Stripe,
    Twilio,
    Sendgrid,
    // infer
    ClaudeApi,
    OpenaiApi,
    Bedrock,
    GeminiApi,
    Vllm,
    Triton,
    // batch
    SparkEmr,
    Dataproc,
    Databricks,
    // media
    Livekit,
    Mediasoup,
    ElementalLive,
    Wowza,
    // coord
    Etcd,
    Zookeeper,
    Consul,
    // vector
    Pgvector,
    Pinecone,
    Qdrant,
    Milvus,
    // warehouse
    Snowflake,
    Bigquery,
    Redshift,
    Clickhouse,
    DatabricksSql,
    // lakehouse
    Iceberg,
    Delta,
    Hudi,
    // tsdb
    Influxdb,
    Timescaledb,
    Timestream,
    Mimir,
    // dbproxy
    Pgbouncer,
    RdsProxy,
    Proxysql,
    Vtgate,
    // graph
    #[serde(rename = "neo4j")]
    Neo4j,
    Neptune,
    Janusgraph,
    // streamproc
    Flink,
    KafkaStreams,
    SparkStreaming,
    // mqtt
    Emqx,
    AwsIotCore,
    Mosquitto,
    Hivemq,
    // connector
    KafkaConnect,
    Debezium,
    Fivetran,
    Airbyte,
    // config
    Launchdarkly,
    AwsAppconfig,
    Unleash,
    // --- M8.3: new flavours on the existing kinds ---
    Scylladb,
    Bigtable,
    CosmosDb,
    Cockroachdb,
    AuroraPostgresql,
    Spanner,
    Postgis,
    Citus,
    Valkey,
    Dragonfly,
    Hazelcast,
    AzureServiceBus,
    AmazonMq,
    SqsFifo,
    Redpanda,
    Pulsar,
    AzureEventHubs,
    Nats,
    AzureWebPubsub,
    Eventbridge,
    Solr,
    Meilisearch,
    AzureBlob,
    CloudflareR2,
    Minio,
    EnvoyLb,
    Traefik,
    GlobalAccelerator,
    Akamai,
    AzureApim,
    Litellm,
    ApolloRouter,
    CloudflareWorkers,
    AzureFunctions,
    AspnetCore,
    Rails,
    Bullmq,
    TemporalWorker,
    Ffmpeg,
    Mediaconvert,
    Temporal,
    Dagster,
    #[serde(rename = "k8s_cronjob")]
    K8sCronjob,
    IotDevice,
    PartnerServer,
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
    Kb,
    Mb,
    Gbps,
    Tokens,
    /// Tokens per second.
    Tps,
    /// Per minute (request and token limits of hosted models).
    PerMin,
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
    /// M8: the name of one of the problem's declared providers (a Third-Party API card's
    /// binding); any short name in the Sandbox.
    Binding,
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
    /// A simulator knob no vendor publishes (a tail multiplier, backlog slots, a managed
    /// service's "effectively unlimited" concurrency); the source cell says why.
    Model,
    /// A rule of thumb rather than a measured figure.
    RuleOfThumb,
}

/// Props whose defaults are calibrated figures rather than design choices.
pub const CALIBRATED_PROPS: &[&str] = &[
    "service_ms",
    "concurrency",
    "concurrency_limit",
    "cold_start_ms",
    "replication_lag_ms",
    "failover_s",
    "visibility_timeout_s",
    "session_timeout_s",
    "query_ratio",
    // M8 kinds: product figures, not design choices.
    "max_connections",
    "conn_memory_kb",
    "msgs_per_s",
    "accept_per_s",
    "login_ms",
    "verify_ms",
    "logins_per_s",
    "ttft_ms",
    "tokens_per_s",
    "prefill_tps",
    "rpm",
    "tpm",
    "task_overhead_ms",
    "streams_per_instance",
    "election_ms",
    "scan_gbps",
    "overhead_ms",
    "points_per_s",
    "series_limit",
    "edges_per_ms",
    "records_per_s",
];

/// Calibration row ids whose source has been confirmed (the planning chat checked these on
/// 2026-11-22 and 2026-11-23; `docs/calibration.md` quotes each source). A row in none of the
/// lists is `source: needed`.
pub const CITED: &[&str] = &[
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
    // Prices cited in the M7b plan review (list prices, us-east-1, read 2026-11-22).
    "dns.cost.million",
    "nosql.cost.million",
    "nosql.cost.million_write",
    "nosql.cost.lane@cassandra",
    "nosql.cost.million@cassandra",
    "nosql.cost.million_write@cassandra",
    "nosql.cost.lane@mongodb",
    "nosql.cost.million@mongodb",
    "nosql.cost.million_write@mongodb",
    "object.cost.million",
    "object.cost.million_write",
    "queue.cost.million",
    "queue.cost.million_write",
    "queue.cost.instance@rabbitmq",
    "queue.cost.million@rabbitmq",
    "queue.cost.million_write@rabbitmq",
    "stream.cost.instance",
    "stream.cost.instance@kinesis",
    "stream.cost.lane@kinesis",
    "stream.cost.million_write@kinesis",
    "stream.cost.unit_kb@kinesis",
    "pubsub.cost.million_write",
    "pubsub.cost.million@google_pubsub",
    "pubsub.cost.million_write@google_pubsub",
    "pubsub.cost.unit_kb@google_pubsub",
    "pubsub.cost.instance@redis_pubsub",
    "pubsub.cost.million_write@redis_pubsub",
    "dlq.cost.million",
    "dlq.cost.million_write",
    // Calibration citations part 2a (M14; list prices read 2026-11-22).
    "waf.service_ms",
    "waf.cost.instance",
    "waf.cost.million",
    "lb.cost.instance",
    "gateway.service_ms",
    "gateway.cost.instance",
    "gateway.cost.million",
    "ratelimit.service_ms",
    "ratelimit.cost.instance",
    "app.cost.instance",
    "micro.cost.instance",
    "worker.concurrency",
    "worker.concurrency@sidekiq",
    "worker.cost.instance",
    "serverless.queue_per_server",
    "sql.replication_lag_ms",
    "nosql.service_ms",
    "nosql.write_slots",
    "cache.servers_per_instance",
    "cache.servers_per_instance@memcached",
    "search.cost.instance",
    "queue.service_ms",
    "stream.service_ms",
    "scheduler.cost.instance",
    "metrics.cost.instance",
    "logs.cost.instance",
    "tracing.cost.instance",
    // Part 2b (M14): M8 kinds and flavours (list prices read 2026-11-22/23).
    "lb.cost.instance@envoy_lb",
    "lb.cost.instance@traefik",
    "lb.cost.instance@global_accelerator",
    "lb.cost.million@global_accelerator",
    "gateway.service_ms@apollo_router",
    "gateway.cost.instance@azure_apim",
    "gateway.cost.instance@litellm",
    "gateway.cost.instance@apollo_router",
    "worker.concurrency@bullmq",
    "worker.concurrency@temporal_worker",
    "worker.cost.million@mediaconvert",
    "serverless.cold_start_ms@cloudflare_workers",
    "serverless.cost.million@cloudflare_workers",
    "serverless.cost.million@azure_functions",
    "sql.max_connections",
    "sql.replication_lag_ms@aurora_postgresql",
    "sql.cost.instance@spanner",
    "nosql.servers_per_instance@scylladb",
    "nosql.cost.lane@scylladb",
    "nosql.cost.lane@bigtable",
    "nosql.cost.million@cosmos_db",
    "nosql.cost.million_write@cosmos_db",
    "cache.servers_per_instance@dragonfly",
    "object.cost.million@azure_blob",
    "object.cost.million_write@azure_blob",
    "object.cost.million@cloudflare_r2",
    "object.cost.million_write@cloudflare_r2",
    "search.service_ms@meilisearch",
    "queue.servers_per_instance@sqs_fifo",
    "queue.service_ms@sqs_fifo",
    "queue.cost.million@azure_service_bus",
    "queue.cost.million_write@azure_service_bus",
    "queue.cost.instance@amazon_mq",
    "queue.cost.million@sqs_fifo",
    "queue.cost.million_write@sqs_fifo",
    "stream.cost.lane@azure_event_hubs",
    "stream.cost.million_write@azure_event_hubs",
    "pubsub.service_ms@nats",
    "pubsub.cost.million_write@azure_web_pubsub",
    "pubsub.cost.million_write@eventbridge",
    "scheduler.cost.instance@dagster",
    "infer.ttft_ms@vllm",
    "infer.tokens_per_s@gemini_api",
    "infer.rpm",
    "infer.tpm",
    "infer.cost.meter.tokens_out@openai_api",
    "infer.cost.instance@vllm",
    "infer.cost.instance@triton",
    "vector.cost.instance",
    "vector.cost.million@pinecone",
    "vector.cost.million_write@pinecone",
    "warehouse.concurrency",
    "warehouse.cost.instance",
    "warehouse.cost.meter.tb_scanned@bigquery",
    "warehouse.cost.instance@clickhouse",
    "lakehouse.cost.gb",
    "tsdb.series_limit",
    "tsdb.cost.million_write@timestream",
    "streamproc.records_per_s",
    "streamproc.cost.instance",
    "connector.cost.instance",
    "connector.cost.million@fivetran",
    "graph.edges_per_ms",
    "coord.election_ms",
    "coord.service_ms",
    "coord.cost.instance",
    "ws.max_connections",
    "ws.max_connections@phoenix",
    "ws.conn_memory_kb",
    "ws.accept_per_s",
    "ws.cost.instance",
    "ws.cost.meter.conn_minutes@aws_apigw_ws",
    "ws.cost.meter.messages@aws_apigw_ws",
    "mqtt.max_connections",
    "mqtt.conn_memory_kb",
    "mqtt.service_ms",
    "mqtt.cost.instance",
    "mqtt.cost.meter.conn_minutes@aws_iot_core",
    "mqtt.cost.meter.messages@aws_iot_core",
    "media.streams_per_instance",
    "media.cost.instance",
    "media.cost.instance@elemental_live",
    "media.cost.instance@wowza",
    "idp.logins_per_s",
    "idp.cost.instance@keycloak",
    "dbproxy.service_ms",
    "dbproxy.cost.instance",
    "dbproxy.cost.instance@rds_proxy",
    "external.cost.million@sendgrid",
];

/// Calibration row ids that are modelling choices (`model: <why>` in `docs/calibration.md`).
pub const MODEL: &[&str] = &[
    // M8 kinds: tail multipliers, backlog slots, and pass-through capacities.
    "ws.servers_per_instance",
    "ws.p99_ratio",
    "ws.queue_per_server",
    "idp.p99_ratio",
    "idp.queue_per_server",
    "infer.p99_ratio",
    "infer.queue_per_server",
    "batch.p99_ratio",
    "batch.queue_per_server",
    "media.p99_ratio",
    "media.queue_per_server",
    "coord.p99_ratio",
    "coord.queue_per_server",
    "vector.p99_ratio",
    "vector.queue_per_server",
    "warehouse.p99_ratio",
    "warehouse.queue_per_server",
    "lakehouse.servers_per_instance",
    "lakehouse.p99_ratio",
    "lakehouse.queue_per_server",
    "tsdb.p99_ratio",
    "tsdb.queue_per_server",
    "dbproxy.servers_per_instance",
    "dbproxy.p99_ratio",
    "dbproxy.queue_per_server",
    "graph.p99_ratio",
    "graph.queue_per_server",
    "streamproc.p99_ratio",
    "streamproc.queue_per_server",
    "mqtt.servers_per_instance",
    "mqtt.p99_ratio",
    "mqtt.queue_per_server",
    "connector.p99_ratio",
    "connector.queue_per_server",
    // Calibration citations part 2a (M14).
    "dns.servers_per_instance",
    "dns.service_ms",
    "dns.p99_ratio",
    "dns.queue_per_server",
    "dns.query_ratio",
    "cdn.servers_per_instance",
    "cdn.service_ms",
    "cdn.p99_ratio",
    "cdn.queue_per_server",
    "waf.servers_per_instance",
    "waf.p99_ratio",
    "waf.queue_per_server",
    "lb.service_ms",
    "lb.service_ms@aws_nlb",
    "lb.servers_per_instance",
    "lb.p99_ratio",
    "lb.queue_per_server",
    "gateway.concurrency",
    "gateway.p99_ratio",
    "gateway.queue_per_server",
    "ratelimit.servers_per_instance",
    "ratelimit.p99_ratio",
    "ratelimit.queue_per_server",
    "app.concurrency",
    "app.service_ms",
    "app.service_ms@go",
    "app.service_ms@django",
    "app.p99_ratio",
    "app.queue_per_server",
    "micro.concurrency",
    "micro.service_ms",
    "micro.p99_ratio",
    "micro.queue_per_server",
    "worker.service_ms",
    "worker.p99_ratio",
    "worker.queue_per_server",
    "serverless.cold_start_ms@cloud_run",
    "serverless.service_ms",
    "serverless.p99_ratio",
    "sql.service_ms",
    "sql.p99_ratio",
    "sql.queue_per_server",
    "nosql.servers_per_instance",
    "nosql.p99_ratio",
    "nosql.queue_per_server",
    "nosql.failover_s",
    "cache.p99_ratio",
    "cache.queue_per_server",
    "object.p99_ratio",
    "object.queue_per_server",
    "search.service_ms",
    "search.p99_ratio",
    "queue.servers_per_instance",
    "queue.p99_ratio",
    "queue.queue_per_server",
    "stream.servers_per_instance",
    "stream.p99_ratio",
    "stream.queue_per_server",
    "pubsub.service_ms",
    "pubsub.servers_per_instance",
    "pubsub.p99_ratio",
    "pubsub.queue_per_server",
    "dlq.servers_per_instance",
    "dlq.service_ms",
    "dlq.p99_ratio",
    "dlq.queue_per_server",
    // Part 2b (M14).
    "cdn.cost.million@akamai",
    "lb.service_ms@global_accelerator",
    "app.service_ms@aspnet_core",
    "app.service_ms@rails",
    "worker.concurrency@ffmpeg",
    "worker.service_ms@ffmpeg",
    "worker.service_ms@mediaconvert",
    "serverless.service_ms@cloudflare_workers",
    "sql.service_ms@postgis",
    "sql.service_ms@citus",
    "cache.servers_per_instance@hazelcast",
    "object.cost.instance@minio",
    "pubsub.cost.instance@nats",
    "infer.ttft_ms@triton",
    "infer.prefill_tps",
    "infer.servers_per_instance",
    "infer.service_ms",
    "vector.service_ms",
    "vector.servers_per_instance",
    "vector.failover_s",
    "warehouse.scan_gbps",
    "warehouse.overhead_ms",
    "warehouse.service_ms",
    "lakehouse.service_ms",
    "tsdb.service_ms",
    "tsdb.servers_per_instance",
    "tsdb.failover_s",
    "streamproc.concurrency",
    "streamproc.service_ms",
    "batch.task_overhead_ms",
    "batch.servers_per_instance",
    "batch.service_ms",
    "connector.concurrency",
    "connector.service_ms",
    "graph.service_ms",
    "graph.failover_s",
    "graph.servers_per_instance",
    "coord.servers_per_instance",
    "ws.msgs_per_s",
    "ws.service_ms",
    "media.streams_per_instance@elemental_live",
    "media.streams_per_instance@wowza",
    "media.servers_per_instance",
    "media.service_ms",
    "idp.login_ms",
    "idp.verify_ms",
    "idp.servers_per_instance",
    "idp.service_ms",
    "config.cost.instance",
];

/// Calibration row ids that are rules of thumb.
pub const RULE_OF_THUMB: &[&str] = &[];

/// A row's status, from the lists above.
pub fn calibration_status(id: &str) -> CalibrationStatus {
    if CITED.contains(&id) {
        CalibrationStatus::Cited
    } else if MODEL.contains(&id) {
        CalibrationStatus::Model
    } else if RULE_OF_THUMB.contains(&id) {
        CalibrationStatus::RuleOfThumb
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
            (PropType::Binding, PropValue::Text(s)) => {
                s.len() <= 40
                    && s.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            }
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
                | (
                    PropType::Choice { .. } | PropType::Binding,
                    PropValue::Text(_)
                )
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

/// A flavour's monthly price (ADR 0006): per instance (or per card), per shard or partition,
/// per GB of cache memory, and per million requests through it, reads and writes apart.
/// List prices (us-east-1, on demand, read 2026-11-22; `docs/calibration.md` cites each) with
/// no free tiers, discounts or data transfer; the judge compares designs with each other.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Cost {
    pub per_instance_month: f64,
    /// Per lane (a stream's partitions as shards, a NoSQL card's partitions as nodes), for
    /// products billed that way. SQL and Search scale `per_instance_month` by their shards
    /// over the default instead (M5.0's cost per shard).
    pub per_lane_month: f64,
    pub per_gb_month: f64,
    /// Per million requests served; for stores and brokers, the reads (GET, read units,
    /// deliveries, the receive of a retried message).
    pub per_million: f64,
    /// Per million writes (PUT, write units, publishes, messages); None bills them as reads.
    pub per_million_write: Option<f64>,
    /// Billing unit size: a request counts `max(1, payload_kb / unit_kb)` units; 0 counts
    /// every request as one.
    pub unit_kb: f64,
    /// Usage meters for products billed by something other than requests (M8).
    #[serde(default)]
    pub meters: Meters,
    /// Bill instance time only while the card is busy (auto-suspending warehouses).
    #[serde(default)]
    pub active_only: bool,
}

/// Prices per unit of usage (M8): tokens, scanned data, connection time, messages. Zero means
/// the product doesn't bill that way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
pub struct Meters {
    /// Per million input (prompt) tokens.
    pub tokens_in_m: f64,
    /// Per million output (generated) tokens.
    pub tokens_out_m: f64,
    /// Per TB scanned.
    pub tb_scanned: f64,
    /// Per million connection-minutes.
    pub conn_minutes_m: f64,
    /// Per million messages sent to connections.
    pub messages_m: f64,
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
        // M8 kinds (their S-features refine these; see ADR 0006).
        K::Ws => d(10_000.0, 0.5, 3.0, 8.0),
        K::Idp => d(256.0, 5.0, 4.0, 8.0),
        K::Infer => d(16.0, 300.0, 3.0, 64.0),
        K::Batch => d(4.0, 1000.0, 3.0, 64.0),
        K::Media => d(500.0, 20.0, 3.0, 4.0),
        K::Coord => d(64.0, 2.0, 4.0, 8.0),
        K::Vector => d(16.0, 2.0, 4.0, 8.0),
        K::Warehouse => d(8.0, 300.0, 3.0, 16.0),
        K::Lakehouse => d(1_000.0, 50.0, 4.0, 4.0),
        K::Tsdb => d(64.0, 1.0, 4.0, 8.0),
        K::Dbproxy => d(10_000.0, 0.3, 3.0, 8.0),
        K::Graph => d(32.0, 1.0, 5.0, 4.0),
        K::Streamproc => d(4.0, 0.05, 3.0, 1_000.0),
        K::Mqtt => d(10_000.0, 2.0, 3.0, 4.0),
        K::Connector => d(4.0, 50.0, 4.0, 8.0),
        K::Client
        | K::Scheduler
        | K::Autoscale
        | K::Metrics
        | K::Logs
        | K::Tracing
        | K::External
        | K::Config => None,
    };
    if let Some(s) = &mut s {
        s.failover_s = match kind {
            K::Sql | K::Graph | K::Tsdb => 30.0,
            K::Cache => 15.0,
            K::Search | K::Vector => 10.0,
            K::Nosql => 5.0,
            _ => 0.0,
        };
    }
    s
}

/// A flavour's own price, where it isn't its kind's.
fn flavour_cost(flavour: Flavour) -> Option<Cost> {
    use Flavour as F;
    let lane = |m: f64| Cost {
        per_lane_month: m,
        per_million_write: Some(0.0),
        ..Cost::default()
    };
    let node = |m: f64| Cost {
        per_instance_month: m,
        per_million_write: Some(0.0),
        ..Cost::default()
    };
    let per_call = |m: f64| Cost {
        per_million: m,
        ..Cost::default()
    };
    let tokens = |i: f64, o: f64| Cost {
        meters: Meters {
            tokens_in_m: i,
            tokens_out_m: o,
            ..Meters::default()
        },
        ..Cost::default()
    };
    Some(match flavour {
        // Self-hosted on EC2 m5.large ($0.096/h × 730); no per-request fee.
        F::Rabbitmq => node(70.0),
        // $0.015 per shard-hour × 730, plus $0.014 per million 25 KB PUT payload units.
        F::Kinesis => Cost {
            per_lane_month: 10.95,
            per_million_write: Some(0.014),
            unit_kb: 25.0,
            ..Cost::default()
        },
        // $40 per TiB, at least 1 KB per publish and per delivery: $0.037 per million each.
        F::GooglePubsub => Cost {
            per_million: 0.037,
            per_million_write: Some(0.037),
            unit_kb: 1.0,
            ..Cost::default()
        },
        // Self-hosted Redis: the cache node price (2 GB at $18/GB-month); no per-message fee.
        F::RedisPubsub => node(36.0),
        // Self-hosted on EC2 i4i.xlarge ($0.343/h × 730) per node (partition).
        F::Cassandra => lane(250.0),
        // MongoDB Atlas M30 ($0.54/h × 730) per shard (partition), a 3-node replica set.
        F::Mongodb => lane(394.0),
        // --- M8 kinds; every figure is `source: needed` until the citations (part 2b). ---
        // API Gateway WebSocket: $1.00 per million messages, $0.25 per million connection-minutes.
        F::AwsApigwWs => Cost {
            meters: Meters {
                messages_m: 1.0,
                conn_minutes_m: 0.25,
                ..Meters::default()
            },
            ..Cost::default()
        },
        F::Keycloak => node(70.0),
        // Twilio SMS ~$0.0079 per message; SendGrid ~$0.0009 per email (teaching prices).
        F::Twilio => per_call(7_900.0),
        F::Sendgrid => per_call(900.0),
        F::Stripe => per_call(0.0),
        // Per million input / output tokens.
        F::OpenaiApi => tokens(2.5, 10.0),
        F::Bedrock => tokens(3.0, 15.0),
        F::GeminiApi => tokens(1.25, 10.0),
        // A g5.xlarge-class GPU node ($1.006/h × 730).
        F::Vllm | F::Triton => node(734.0),
        F::ElementalLive => node(2_190.0),
        F::Wowza => node(250.0),
        // Pinecone serverless: read and write units.
        F::Pinecone => Cost {
            per_million: 16.0,
            per_million_write: Some(4.0),
            ..Cost::default()
        },
        // BigQuery on demand: $6.25 per TB scanned, no instance.
        F::Bigquery => Cost {
            meters: Meters {
                tb_scanned: 6.25,
                ..Meters::default()
            },
            ..Cost::default()
        },
        F::Redshift => node(182.0),
        F::Clickhouse => node(250.0),
        F::DatabricksSql => Cost {
            per_instance_month: 1_000.0,
            active_only: true,
            ..Cost::default()
        },
        F::Timestream => Cost {
            per_million_write: Some(0.5),
            per_million: 0.01,
            ..Cost::default()
        },
        F::RdsProxy => node(45.0),
        F::Neptune => node(350.0),
        // AWS IoT Core: $1.00 per million messages, $0.08 per million connection-minutes.
        F::AwsIotCore => Cost {
            meters: Meters {
                messages_m: 1.0,
                conn_minutes_m: 0.08,
                ..Meters::default()
            },
            ..Cost::default()
        },
        F::Fivetran => per_call(500.0),
        // --- M8.3 flavours on existing kinds (teaching prices; `source: needed`). ---
        // ScyllaDB self-hosted on the same i4i.xlarge node as Cassandra.
        F::Scylladb => lane(250.0),
        // Bigtable: $0.65 per node-hour × 730 per node (partition).
        F::Bigtable => lane(474.0),
        // Cosmos DB serverless request units, shaped as reads and writes.
        F::CosmosDb => Cost {
            per_million: 0.25,
            per_million_write: Some(1.25),
            ..Cost::default()
        },
        F::Cockroachdb => node(438.0),
        F::AuroraPostgresql => node(300.0),
        // Spanner: one node ($0.90/h × 730).
        F::Spanner => node(657.0),
        // Service Bus Standard: about $0.80 per million operations, three per message.
        F::AzureServiceBus => Cost {
            per_million: 0.8,
            per_million_write: Some(2.4),
            ..Cost::default()
        },
        // Amazon MQ mq.m5.large ($0.288/h × 730).
        F::AmazonMq => node(210.0),
        // SQS FIFO: $0.50 per million requests, three per message.
        F::SqsFifo => Cost {
            per_million: 0.5,
            per_million_write: Some(1.5),
            ..Cost::default()
        },
        F::Redpanda | F::Pulsar => node(153.0),
        // Event Hubs Standard: a throughput unit per partition ($0.03/h × 730) + $0.028 per
        // million events.
        F::AzureEventHubs => Cost {
            per_lane_month: 21.9,
            per_million_write: Some(0.028),
            ..Cost::default()
        },
        F::Nats => node(70.0),
        F::AzureWebPubsub | F::Eventbridge => Cost {
            per_million_write: Some(1.0),
            ..Cost::default()
        },
        F::Meilisearch => node(150.0),
        // Azure Blob hot: reads $0.44, writes $5.50 per million.
        F::AzureBlob => Cost {
            per_million: 0.44,
            per_million_write: Some(5.5),
            ..Cost::default()
        },
        // Cloudflare R2: class B $0.36, class A $4.50 per million; no egress fee.
        F::CloudflareR2 => Cost {
            per_million: 0.36,
            per_million_write: Some(4.5),
            ..Cost::default()
        },
        F::Minio => node(70.0),
        F::EnvoyLb | F::Traefik => node(70.0),
        // Global Accelerator: $0.025 per hour, plus its data-transfer premium as a price per
        // request (about 100 KB at $0.015/GB; a placeholder).
        F::GlobalAccelerator => Cost {
            per_instance_month: 18.0,
            per_million: 1.5,
            ..Cost::default()
        },
        F::Akamai => per_call(1.2),
        // API Management Standard unit.
        F::AzureApim => node(686.0),
        F::Litellm | F::ApolloRouter => node(70.0),
        F::CloudflareWorkers => per_call(0.3),
        F::AzureFunctions => per_call(0.4),
        // MediaConvert: per minute of output, here per job.
        F::Mediaconvert => per_call(7_500.0),
        F::Temporal => node(200.0),
        F::Dagster => node(100.0),
        F::K8sCronjob => node(0.0),
        // GCS Standard: Class B $0.40, Class A $5.00 per million, as S3.
        _ => return None,
    })
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
        // M8.3: ScyllaDB's shard-per-core design (vendor benchmarks: several times
        // Cassandra's throughput per node; source needed).
        F::Scylladb => s.servers_per_instance = 96.0,
        // Multi-threaded caches.
        F::Dragonfly | F::Hazelcast => s.servers_per_instance = 8.0,
        // SQS FIFO: 300 messages a second per action without batching.
        F::SqsFifo => {
            s.service_ms = 20.0;
            s.servers_per_instance = 300.0;
        }
        F::Redpanda => s.service_ms = 1.0,
        F::Eventbridge => s.service_ms = 20.0,
        _ => {}
    }
}

/// The kind's default price (flavours depart from it in `flavour_cost`).
fn kind_cost(kind: ComponentKind) -> Cost {
    use ComponentKind as K;
    let c = |i: f64, g: f64, m: f64| Cost {
        per_instance_month: i,
        per_gb_month: g,
        per_million: m,
        ..Cost::default()
    };
    // Reads and writes priced apart.
    let rw = |read: f64, write: f64| Cost {
        per_million: read,
        per_million_write: Some(write),
        ..Cost::default()
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
        // DynamoDB on demand: $0.125 per million read units, $0.625 per million write units.
        K::Nosql => rw(0.125, 0.625),
        K::Cache => c(0.0, 18.0, 0.0),
        // S3: GET $0.40, PUT $5.00 per million.
        K::Object => rw(0.4, 5.0),
        K::Search => c(250.0, 0.0, 0.0),
        // SQS: $0.40 per million requests, three per message (send, receive, delete); a
        // retry is one more receive.
        K::Queue | K::Dlq => rw(0.4, 1.2),
        // Kafka on MSK: kafka.m5.large at $0.21/h × 730 per broker; no per-request fee.
        K::Stream => c(153.0, 0.0, 0.0),
        // SNS: $0.50 per million publishes; deliveries to queues and functions are free.
        K::Pubsub => rw(0.0, 0.5),
        K::Scheduler => c(20.0, 0.0, 0.0),
        K::Metrics => c(100.0, 0.0, 0.0),
        K::Logs => c(150.0, 0.0, 0.0),
        K::Tracing => c(80.0, 0.0, 0.0),
        // --- M8 kinds (teaching prices; `source: needed` until the citations). ---
        K::Ws | K::Coord | K::Streamproc | K::Mqtt | K::Connector | K::Media => c(70.0, 0.0, 0.0),
        // Managed identity: per million logins (an MAU proxy).
        K::Idp => c(0.0, 0.0, 5.0),
        K::External => c(0.0, 0.0, 0.0),
        // Hosted models: per million input and output tokens (Claude Sonnet-class list prices).
        K::Infer => Cost {
            meters: Meters {
                tokens_in_m: 3.0,
                tokens_out_m: 15.0,
                ..Meters::default()
            },
            ..Cost::default()
        },
        // An m5.xlarge-class node ($0.192/h × 730).
        K::Batch => c(140.0, 0.0, 0.0),
        K::Vector | K::Tsdb | K::Graph => c(260.0, 0.0, 0.0),
        // Snowflake X-Small: 1 credit/h at $2 × 730, billed only while running.
        K::Warehouse => Cost {
            per_instance_month: 1_460.0,
            active_only: true,
            ..Cost::default()
        },
        // Object storage behind the table (S3 Standard $0.023/GB-month).
        K::Lakehouse => c(0.0, 0.023, 0.0),
        K::Dbproxy => c(30.0, 0.0, 0.0),
        K::Config => c(20.0, 0.0, 0.0),
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
    /// A zone's service-mesh policy props (v4); their defaults leave the mesh's retries,
    /// timeouts and circuit breaking off.
    pub mesh_props: Vec<PropSpec>,
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
            f.cost = flavour_cost(f.flavour).unwrap_or_else(|| kind_cost(kind));
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
/// S8 (M8): how a service protects itself from its dependencies and from overload. Every
/// default is off, the behaviour before M8.
fn resilience() -> Vec<PropSpec> {
    vec![
        int("call_timeout_ms", 0.0, 60_000.0, 0.0, Unit::Ms),
        int("breaker_errors_pct", 0.0, 100.0, 0.0, Unit::Percent),
        int("breaker_open_s", 1.0, 600.0, 30.0, Unit::S),
        boolean("bulkhead", false),
        shed_queue(),
        // How a new build reaches the instances: in stages, with a wait between them.
        int("rollout_percent", 1.0, 100.0, 100.0, Unit::Percent),
        int("bake_s", 0.0, 3600.0, 0.0, Unit::S),
        boolean("auto_rollback", false),
    ]
}
/// S7: `manual` (the behaviour before M8: a replica takes over only when the primary dies)
/// or `auto` (a replica cut off from its primary by a split promotes itself: split-brain).
fn split_failover() -> PropSpec {
    choice("split_failover", &["manual", "auto"], "manual")
}
fn shed_queue() -> PropSpec {
    int("shed_queue", 0.0, 1_000_000.0, 0.0, Unit::Count)
}

/// The registry. Order is the library's order: categories, then kinds within each.
pub fn registry() -> Registry {
    use ComponentKind as K;
    use Flavour as F;
    let kinds = vec![
        // --- Clients & Edge ---
        kind(
            K::Client,
            vec![
                fl(F::Web),
                fl(F::Mobile),
                fl(F::IotDevice),
                fl(F::PartnerServer),
            ],
            vec![
                int("rps", 1.0, 1_000_000.0, 1000.0, Unit::Rps),
                // S8 (M8): how a client gives up and tries again. The defaults are the
                // behaviour before M8: one try, a 5 s timeout.
                int("timeout_ms", 100.0, 120_000.0, 5_000.0, Unit::Ms),
                int("retries", 0.0, 10.0, 0.0, Unit::Count),
                int("backoff_ms", 0.0, 60_000.0, 100.0, Unit::Ms),
                choice("jitter", &["none", "full"], "none"),
                int("retry_budget_pct", 0.0, 100.0, 0.0, Unit::Percent),
                // S1: long-lived connections reconnect spread over this window after a drop
                // (0: all at once, a storm), and retry a refused connection after the backoff.
                int("reconnect_jitter_s", 0.0, 600.0, 0.0, Unit::S),
                int("reconnect_backoff_s", 0.0, 600.0, 1.0, Unit::S),
            ],
        ),
        kind(
            K::Dns,
            vec![fl(F::Route53), fl(F::CloudflareDns)],
            vec![
                int("ttl_s", 0.0, 86_400.0, 300.0, Unit::S),
                // S8: `failover` answers with the first healthy target; clients cache the
                // answer for the TTL. `round_robin` spreads over every healthy target.
                choice("routing", &["round_robin", "failover"], "round_robin"),
                // Billing only: the share of requests that reach the DNS service as a query
                // (clients and resolvers cache answers for the TTL).
                float("query_ratio", 0.001, 1.0, 0.001, 0.01, Unit::None),
            ],
        ),
        kind(
            K::Cdn,
            vec![
                fl(F::Cloudfront),
                fl(F::CloudflareCdn),
                fl(F::Fastly),
                fl(F::Akamai),
            ],
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
                fl(F::EnvoyLb),
                fl(F::Traefik),
                fl_with(F::GlobalAccelerator, &[("service_ms", num(0.2))]),
            ],
            vec![
                int("instances", 1.0, 16.0, 1.0, Unit::Count),
                choice(
                    "algorithm",
                    &["round_robin", "least_conn", "key_hash"],
                    "round_robin",
                ),
                service_ms(0.5),
                // S8: eject targets whose queue wait passes this (0 = liveness checks only),
                // keeping at least this share in rotation.
                int("health_timeout_ms", 0.0, 60_000.0, 0.0, Unit::Ms),
                int("min_healthy_pct", 0.0, 100.0, 0.0, Unit::Percent),
            ],
        ),
        kind(
            K::Gateway,
            vec![
                fl(F::Kong),
                fl(F::AwsApiGateway),
                fl(F::AzureApim),
                fl(F::Litellm),
                fl_with(F::ApolloRouter, &[("service_ms", num(3.0))]),
            ],
            [compute_props(2.0, 256.0, 2.0), vec![shed_queue()]].concat(),
        ),
        kind(
            K::Ratelimit,
            vec![fl(F::Envoy), fl(F::RedisTokenBucket)],
            vec![
                int("limit_rps", 1.0, 1_000_000.0, 1000.0, Unit::Rps),
                int("burst", 1.0, 100_000.0, 100.0, Unit::Count),
                // M10: when the limiter can't reach its state (a dead card), refuse every
                // request (`closed`) or let them through unlimited (`open`).
                choice("on_failure", &["closed", "open"], "closed"),
                // H2: count requests, or tokens (prompt + answer) for an LLM gateway; with
                // tokens, the limit and burst are tokens per second and tokens.
                choice("unit", &["requests", "tokens"], "requests"),
            ],
        ),
        kind(
            K::Ws,
            vec![
                fl(F::SocketIo),
                fl(F::AwsApigwWs),
                fl_with(F::Phoenix, &[("max_connections", num(1_000_000.0))]),
            ],
            vec![
                instances(2.0),
                int(
                    "max_connections",
                    1000.0,
                    2_000_000.0,
                    100_000.0,
                    Unit::Count,
                ),
                float("conn_memory_kb", 1.0, 512.0, 1.0, 20.0, Unit::Kb),
                float("memory_gb", 0.5, 1024.0, 0.5, 8.0, Unit::Gb),
                int("msgs_per_s", 100.0, 2_000_000.0, 50_000.0, Unit::Rps),
                int("accept_per_s", 10.0, 100_000.0, 2_000.0, Unit::Rps),
                service_ms(0.5),
            ],
        ),
        kind(
            K::Idp,
            vec![fl(F::Auth0), fl(F::Cognito), fl(F::Keycloak), fl(F::Okta)],
            vec![
                instances(1.0),
                float("login_ms", 1.0, 10_000.0, 1.0, 150.0, Unit::Ms),
                float("verify_ms", 0.1, 1000.0, 0.1, 5.0, Unit::Ms),
                int("logins_per_s", 1.0, 100_000.0, 100.0, Unit::Rps),
                choice("verify", &["remote", "local"], "remote"),
            ],
        ),
        kind(
            K::External,
            vec![
                fl(F::Generic),
                fl(F::Stripe),
                fl(F::Twilio),
                fl(F::Sendgrid),
            ],
            vec![PropSpec {
                key: "binding".into(),
                ty: PropType::Binding,
                default: PropValue::Text(String::new()),
                unit: Unit::None,
                calibration: None,
            }],
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
                fl_with(F::AspnetCore, &[("service_ms", num(6.0))]),
                fl_with(
                    F::Rails,
                    &[("service_ms", num(25.0)), ("concurrency", num(5.0))],
                ),
            ],
            [compute_props(2.0, 64.0, 10.0), resilience()].concat(),
        ),
        kind(
            K::Micro,
            vec![fl(F::Generic)],
            [compute_props(2.0, 64.0, 10.0), resilience()].concat(),
        ),
        kind(
            K::Worker,
            vec![
                fl(F::Celery),
                fl_with(F::Sidekiq, &[("concurrency", num(5.0))]),
                fl_with(F::Bullmq, &[("concurrency", num(10.0))]),
                fl_with(F::TemporalWorker, &[("concurrency", num(100.0))]),
                fl_with(
                    F::Ffmpeg,
                    &[("service_ms", num(10_000.0)), ("concurrency", num(2.0))],
                ),
                fl_with(
                    F::Mediaconvert,
                    &[("service_ms", num(10_000.0)), ("concurrency", num(1000.0))],
                ),
            ],
            [
                vec![
                    instances(2.0),
                    int("concurrency", 1.0, 1024.0, 8.0, Unit::Count),
                    service_ms(100.0),
                ],
                resilience(),
            ]
            .concat(),
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
                fl_with(
                    F::CloudflareWorkers,
                    &[("cold_start_ms", num(5.0)), ("service_ms", num(10.0))],
                ),
                fl(F::AzureFunctions),
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
        kind(
            K::Infer,
            vec![
                fl(F::ClaudeApi),
                fl_with(
                    F::OpenaiApi,
                    &[("ttft_ms", num(500.0)), ("tokens_per_s", num(90.0))],
                ),
                fl(F::Bedrock),
                fl_with(
                    F::GeminiApi,
                    &[("ttft_ms", num(500.0)), ("tokens_per_s", num(120.0))],
                ),
                fl_with(F::Vllm, &[("ttft_ms", num(200.0))]),
                fl_with(
                    F::Triton,
                    &[
                        ("model_type", PropValue::Text("classify".into())),
                        ("ttft_ms", num(5.0)),
                        ("batch_slots", num(64.0)),
                    ],
                ),
            ],
            vec![
                choice("model_type", &["chat", "embed", "classify"], "chat"),
                choice("api_mode", &["realtime", "batch"], "realtime"),
                instances(2.0),
                int("batch_slots", 1.0, 256.0, 16.0, Unit::Count),
                float("ttft_ms", 1.0, 10_000.0, 1.0, 600.0, Unit::Ms),
                float("tokens_per_s", 1.0, 5_000.0, 1.0, 80.0, Unit::Tps),
                int("prefill_tps", 100.0, 100_000.0, 5_000.0, Unit::Tps),
                int("rpm", 1.0, 1_000_000.0, 4_000.0, Unit::PerMin),
                int("tpm", 1_000.0, 100_000_000.0, 400_000.0, Unit::PerMin),
            ],
        ),
        kind(
            K::Batch,
            vec![fl(F::SparkEmr), fl(F::Dataproc), fl(F::Databricks)],
            vec![
                int("instances", 1.0, 256.0, 4.0, Unit::Count),
                int("executors", 1.0, 64.0, 4.0, Unit::Count),
                float("task_overhead_ms", 1.0, 60_000.0, 1.0, 200.0, Unit::Ms),
                boolean("speculation", false),
                int("salting", 1.0, 64.0, 1.0, Unit::Count),
                int("spot_share", 0.0, 100.0, 0.0, Unit::Percent),
                boolean("checkpoint", false),
            ],
        ),
        kind(
            K::Media,
            vec![
                fl(F::Livekit),
                fl(F::Mediasoup),
                fl_with(
                    F::ElementalLive,
                    &[
                        ("mode", PropValue::Text("transcode".into())),
                        ("streams_per_instance", num(8.0)),
                    ],
                ),
                fl_with(
                    F::Wowza,
                    &[
                        ("mode", PropValue::Text("transcode".into())),
                        ("streams_per_instance", num(8.0)),
                    ],
                ),
            ],
            vec![
                instances(2.0),
                choice("mode", &["sfu", "transcode"], "sfu"),
                int("streams_per_instance", 1.0, 10_000.0, 500.0, Unit::Count),
                int("renditions", 1.0, 8.0, 4.0, Unit::Count),
                float("segment_s", 0.5, 10.0, 0.5, 2.0, Unit::S),
                float("bandwidth_gbps", 1.0, 100.0, 1.0, 10.0, Unit::Gbps),
            ],
        ),
        // --- Data ---
        kind(
            K::Sql,
            vec![
                fl(F::Postgresql),
                fl_with(
                    F::Mysql,
                    &[
                        ("concurrency", num(151.0)),
                        ("max_connections", num(1_365.0)),
                    ],
                ),
                fl_with(F::Cockroachdb, &[("service_ms", num(12.0))]),
                fl_with(F::AuroraPostgresql, &[("replication_lag_ms", num(20.0))]),
                fl_with(F::Spanner, &[("service_ms", num(12.0))]),
                fl_with(F::Postgis, &[("service_ms", num(10.0))]),
                fl_with(F::Citus, &[("service_ms", num(5.5)), ("shards", num(4.0))]),
            ],
            vec![
                int("shards", 1.0, 64.0, 1.0, Unit::Count),
                int("concurrency", 1.0, 1000.0, 100.0, Unit::Count),
                service_ms(5.0),
                float("replication_lag_ms", 0.0, 10_000.0, 1.0, 50.0, Unit::Ms),
                int("failover_s", 1.0, 600.0, 30.0, Unit::S),
                choice("read_from", &["any", "primary"], "any"),
                int("cdc_buffer_s", 0.0, 3600.0, 300.0, Unit::S),
                split_failover(),
                boolean("fencing", false),
                // S5: idempotent writes (upsert by key): a replayed result isn't a duplicate.
                boolean("dedup", false),
                // M9: a write that checks and sets in one statement (a conditional UPDATE, a
                // condition expression): on a contended key a race is refused cleanly (S7).
                boolean("conditional_writes", false),
                // M10: a commit waits for replicas: none (async), one replica (`sync`), or a
                // majority of the cluster (`quorum`).
                choice("sync", &["none", "sync", "quorum"], "none"),
                // M8: enforced when a problem turns connection limits on.
                int("max_connections", 1.0, 100_000.0, 1_700.0, Unit::Count),
            ],
        ),
        kind(
            K::Nosql,
            vec![
                fl(F::Dynamodb),
                fl(F::Cassandra),
                fl(F::Mongodb),
                fl(F::Scylladb),
                fl(F::Bigtable),
                fl(F::CosmosDb),
            ],
            vec![
                int("partitions", 1.0, 1024.0, 4.0, Unit::Count),
                int("replication_factor", 1.0, 5.0, 3.0, Unit::Count),
                choice("consistency", &["eventual", "strong"], "eventual"),
                service_ms(3.0),
                int("cdc_buffer_s", 0.0, 3600.0, 300.0, Unit::S),
                split_failover(),
                boolean("fencing", false),
                // S5: idempotent writes (upsert by key): a replayed result isn't a duplicate.
                boolean("dedup", false),
                // M9: a write that checks and sets in one statement (a conditional UPDATE, a
                // condition expression): on a contended key a race is refused cleanly (S7).
                boolean("conditional_writes", false),
            ],
        ),
        kind(
            K::Cache,
            vec![
                fl(F::Redis),
                fl(F::Memcached),
                fl(F::Valkey),
                fl(F::Dragonfly),
                fl(F::Hazelcast),
            ],
            vec![
                float("memory_gb", 0.5, 1024.0, 0.5, 2.0, Unit::Gb),
                int("ttl_s", 0.0, 604_800.0, 300.0, Unit::S),
                choice("eviction", &["lru", "lfu", "ttl"], "lru"),
                boolean("persistence", false),
                service_ms(0.01),
                int("failover_s", 1.0, 600.0, 15.0, Unit::S),
                // S3: `semantic` answers near-repeat questions (an embedding lookup); its hit
                // rate comes from the traffic, never from the design.
                choice("mode", &["exact", "semantic"], "exact"),
                // S8d: answer from an expired copy (up to this long after its TTL) when the
                // origin fails; and coalesce concurrent misses for one key.
                int("serve_stale_s", 0.0, 86_400.0, 0.0, Unit::S),
                boolean("stampede_guard", false),
                // M9: a write that checks and sets in one statement (a conditional UPDATE, a
                // condition expression): on a contended key a race is refused cleanly (S7).
                boolean("conditional_writes", false),
            ],
        ),
        kind(
            K::Object,
            vec![
                fl(F::S3),
                fl(F::Gcs),
                fl(F::AzureBlob),
                fl(F::CloudflareR2),
                fl(F::Minio),
            ],
            vec![service_ms(50.0)],
        ),
        kind(
            K::Search,
            vec![
                fl(F::Elasticsearch),
                fl(F::Opensearch),
                fl(F::Solr),
                fl_with(F::Meilisearch, &[("service_ms", num(5.0))]),
            ],
            vec![
                int("shards", 1.0, 64.0, 2.0, Unit::Count),
                int("replicas", 0.0, 5.0, 1.0, Unit::Count),
                service_ms(20.0),
            ],
        ),
        kind(
            K::Coord,
            vec![fl(F::Etcd), fl(F::Zookeeper), fl(F::Consul)],
            vec![
                int("instances", 1.0, 7.0, 3.0, Unit::Count),
                int("election_ms", 100.0, 30_000.0, 1_000.0, Unit::Ms),
                int("lease_s", 1.0, 600.0, 10.0, Unit::S),
                service_ms(2.0),
            ],
        ),
        kind(
            K::Vector,
            vec![
                fl(F::Pgvector),
                fl(F::Pinecone),
                fl(F::Qdrant),
                fl(F::Milvus),
            ],
            vec![
                int("shards", 1.0, 64.0, 1.0, Unit::Count),
                float("vectors_m", 0.01, 10_000.0, 0.01, 10.0, Unit::Count),
                int("dims", 64.0, 4096.0, 768.0, Unit::Count),
                service_ms(2.0),
            ],
        ),
        kind(
            K::Warehouse,
            vec![
                fl(F::Snowflake),
                fl(F::Bigquery),
                fl(F::Redshift),
                fl(F::Clickhouse),
                fl(F::DatabricksSql),
            ],
            vec![
                instances(1.0),
                concurrency(8.0),
                float("scan_gbps", 0.05, 50.0, 0.05, 1.0, Unit::Gbps),
                float("overhead_ms", 1.0, 60_000.0, 1.0, 300.0, Unit::Ms),
                boolean("result_cache", true),
                int("auto_suspend_s", 0.0, 3600.0, 60.0, Unit::S),
                boolean("dedup", false),
            ],
        ),
        kind(
            K::Lakehouse,
            vec![fl(F::Iceberg), fl(F::Delta), fl(F::Hudi)],
            vec![
                int("target_file_mb", 1.0, 2048.0, 128.0, Unit::Mb),
                choice("partitioning", &["none", "day", "hour"], "none"),
                choice("compaction", &["none", "hourly", "daily"], "daily"),
                float("stored_tb", 0.01, 10_000.0, 0.01, 10.0, Unit::None),
                // H3: writes are merged by key (a MERGE / upsert on commit), so a replay
                // after a crash doesn't add duplicate rows.
                boolean("dedup", false),
            ],
        ),
        kind(
            K::Tsdb,
            vec![
                fl(F::Influxdb),
                fl(F::Timescaledb),
                fl(F::Timestream),
                fl(F::Mimir),
            ],
            vec![
                instances(1.0),
                int("points_per_s", 1_000.0, 10_000_000.0, 500_000.0, Unit::Rps),
                int(
                    "series_limit",
                    1_000.0,
                    100_000_000.0,
                    1_000_000.0,
                    Unit::Count,
                ),
                int("retention_days", 1.0, 3650.0, 30.0, Unit::Days),
                float("memory_gb", 0.5, 1024.0, 0.5, 16.0, Unit::Gb),
                service_ms(1.0),
                boolean("dedup", false),
            ],
        ),
        kind(
            K::Dbproxy,
            vec![
                fl(F::Pgbouncer),
                fl(F::RdsProxy),
                fl(F::Proxysql),
                fl(F::Vtgate),
            ],
            vec![
                instances(1.0),
                int("pool_size", 1.0, 5000.0, 100.0, Unit::Count),
                choice("mode", &["transaction", "session"], "transaction"),
                int("client_limit", 1.0, 100_000.0, 10_000.0, Unit::Count),
                service_ms(0.3),
            ],
        ),
        kind(
            K::Graph,
            vec![fl(F::Neo4j), fl(F::Neptune), fl(F::Janusgraph)],
            vec![
                instances(1.0),
                float("edges_per_ms", 1.0, 100_000.0, 1.0, 500.0, Unit::Count),
                service_ms(1.0),
                int("failover_s", 1.0, 600.0, 30.0, Unit::S),
            ],
        ),
        // --- Async ---
        kind(
            K::Queue,
            vec![
                fl(F::Rabbitmq),
                fl(F::Sqs),
                fl(F::AzureServiceBus),
                fl(F::AmazonMq),
                fl(F::SqsFifo),
            ],
            vec![
                int("max_depth", 1.0, 10_000_000.0, 100_000.0, Unit::Count),
                int("visibility_timeout_s", 1.0, 43_200.0, 30.0, Unit::S),
                int("max_retries", 0.0, 20.0, 3.0, Unit::Count),
                // M10: dispatch live messages before bulk ones (request types marked `bulk`).
                boolean("priority", false),
            ],
        ),
        kind(
            K::Stream,
            vec![
                fl(F::Kafka),
                fl(F::Kinesis),
                fl(F::Redpanda),
                fl(F::Pulsar),
                fl(F::AzureEventHubs),
            ],
            vec![
                int("partitions", 1.0, 1024.0, 6.0, Unit::Count),
                int("session_timeout_s", 1.0, 1800.0, 45.0, Unit::S),
                int("retention_h", 1.0, 8760.0, 168.0, Unit::H),
                // M10: dispatch live records before bulk ones (request types marked `bulk`).
                boolean("priority", false),
            ],
        ),
        kind(
            K::Pubsub,
            vec![
                fl(F::GooglePubsub),
                fl(F::Sns),
                fl(F::RedisPubsub),
                fl_with(F::Nats, &[("service_ms", num(0.5))]),
                fl(F::AzureWebPubsub),
                fl(F::Eventbridge),
            ],
            vec![service_ms(5.0)],
        ),
        kind(
            K::Dlq,
            vec![fl(F::Generic)],
            vec![int("retention_days", 1.0, 14.0, 4.0, Unit::Days)],
        ),
        kind(
            K::Scheduler,
            vec![
                fl(F::Cron),
                fl(F::Airflow),
                fl(F::Temporal),
                fl(F::Dagster),
                fl(F::K8sCronjob),
            ],
            vec![int("interval_s", 1.0, 86_400.0, 60.0, Unit::S)],
        ),
        kind(
            K::Streamproc,
            vec![fl(F::Flink), fl(F::KafkaStreams), fl(F::SparkStreaming)],
            vec![
                instances(2.0),
                int("concurrency", 1.0, 256.0, 4.0, Unit::Count),
                int("records_per_s", 100.0, 10_000_000.0, 20_000.0, Unit::Rps),
                choice("window", &["none", "tumbling", "sliding"], "none"),
                int("window_s", 1.0, 86_400.0, 60.0, Unit::S),
                int("slide_s", 1.0, 3600.0, 10.0, Unit::S),
                int("watermark_s", 0.0, 3600.0, 5.0, Unit::S),
                int("allowed_lateness_s", 0.0, 86_400.0, 0.0, Unit::S),
                int("checkpoint_s", 1.0, 3600.0, 60.0, Unit::S),
                float("state_kb", 0.1, 1024.0, 0.1, 1.0, Unit::Kb),
                float("memory_gb", 0.5, 1024.0, 0.5, 8.0, Unit::Gb),
            ],
        ),
        kind(
            K::Mqtt,
            vec![
                fl(F::Emqx),
                fl(F::AwsIotCore),
                fl(F::Mosquitto),
                fl(F::Hivemq),
            ],
            vec![
                instances(2.0),
                int(
                    "max_connections",
                    1000.0,
                    10_000_000.0,
                    1_000_000.0,
                    Unit::Count,
                ),
                float("conn_memory_kb", 1.0, 512.0, 1.0, 8.0, Unit::Kb),
                int("msgs_per_s", 100.0, 10_000_000.0, 100_000.0, Unit::Rps),
                choice("qos", &["at_most_once", "at_least_once"], "at_most_once"),
                service_ms(2.0),
            ],
        ),
        kind(
            K::Connector,
            vec![
                fl(F::KafkaConnect),
                fl(F::Debezium),
                fl(F::Fivetran),
                fl(F::Airbyte),
            ],
            vec![
                instances(1.0),
                int("concurrency", 1.0, 256.0, 4.0, Unit::Count),
                int("poll_s", 1.0, 86_400.0, 300.0, Unit::S),
                int("api_rps", 1.0, 100_000.0, 10.0, Unit::Rps),
                service_ms(50.0),
            ],
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
        kind(
            K::Config,
            vec![fl(F::Launchdarkly), fl(F::AwsAppconfig), fl(F::Unleash)],
            vec![
                choice("delivery", &["stream", "poll"], "stream"),
                int("poll_s", 1.0, 3600.0, 30.0, Unit::S),
                boolean("staged", false),
                int("stages", 1.0, 10.0, 3.0, Unit::Count),
                int("bake_s", 0.0, 3600.0, 60.0, Unit::S),
                boolean("kill_switch", false),
            ],
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
        mesh_props: vec![
            int("mesh_retries", 0.0, 10.0, 0.0, Unit::Count),
            int("mesh_timeout_ms", 0.0, 60_000.0, 0.0, Unit::Ms),
            int("mesh_breaker_errors_pct", 0.0, 100.0, 0.0, Unit::Percent),
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
/// - prices: `<kind>.cost.instance|lane|gb|million|million_write|unit_kb` and
///   `<kind>.cost.meter.<meter>` when non-zero,
///   `@<flavour>` where a flavour departs from its kind.
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
                ("lane", c.per_lane_month),
                ("gb", c.per_gb_month),
                ("million", c.per_million),
                ("million_write", c.per_million_write.unwrap_or(0.0)),
                ("unit_kb", c.unit_kb),
                ("meter.tokens_in", c.meters.tokens_in_m),
                ("meter.tokens_out", c.meters.tokens_out_m),
                ("meter.tb_scanned", c.meters.tb_scanned),
                ("meter.conn_minutes", c.meters.conn_minutes_m),
                ("meter.messages", c.meters.messages_m),
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
    fn has_42_kinds_in_5_categories_in_library_order() {
        let r = registry();
        assert_eq!(r.kinds.len(), 42);
        let kinds: Vec<_> = r.kinds.iter().map(|k| k.kind).collect();
        assert_eq!(kinds, ComponentKind::ALL.to_vec());
        let per_category: Vec<usize> = Category::ALL
            .iter()
            .map(|c| r.kinds.iter().filter(|k| k.category == *c).count())
            .collect();
        assert_eq!(per_category, vec![10, 8, 12, 8, 4]);
    }

    #[test]
    fn ids_icons_and_tokens_are_consistent() {
        let r = registry();
        let icons: HashSet<_> = r.kinds.iter().map(|k| k.icon.as_str()).collect();
        assert_eq!(icons.len(), 42, "every kind has its own glyph");
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

    /// B2 (M7b): every flavour resolves to exactly one way of billing its capacity. A
    /// per-lane price belongs to kinds whose lanes are real shards (NoSQL partitions, stream
    /// partitions as Kinesis shards) and replaces the per-instance price there; SQL and
    /// Search keep M5.0's per-instance price scaled by shards, so they have no lane price.
    #[test]
    fn every_flavour_has_one_capacity_rule() {
        use ComponentKind as K;
        for k in &registry().kinds {
            for f in &k.flavours {
                let c = f.cost;
                let id = format!("{}@{}", k.kind.id(), flavour_id(f.flavour));
                if c.per_lane_month > 0.0 {
                    assert!(
                        matches!(k.kind, K::Nosql | K::Stream),
                        "{id}: a lane price on a kind without billed lanes"
                    );
                    assert_eq!(
                        c.per_instance_month, 0.0,
                        "{id}: billed per lane and per instance"
                    );
                }
                if matches!(k.kind, K::Sql | K::Search) {
                    assert_eq!(
                        c.per_lane_month, 0.0,
                        "{id}: SQL/Search scale the instance price"
                    );
                }
                assert!(
                    c.unit_kb >= 0.0 && c.per_million >= 0.0,
                    "{id}: negative price"
                );
                assert!(
                    c.per_million_write.unwrap_or(0.0) >= 0.0,
                    "{id}: negative price"
                );
            }
        }
        // The cited rules, spot-checked.
        let cost = |k: K, f: Flavour| registry().kind(k).flavour(f).unwrap().cost;
        assert_eq!(
            cost(K::Nosql, Flavour::Dynamodb).per_million_write,
            Some(0.625)
        );
        assert_eq!(cost(K::Nosql, Flavour::Cassandra).per_lane_month, 250.0);
        assert_eq!(cost(K::Stream, Flavour::Kafka).per_instance_month, 153.0);
        assert_eq!(cost(K::Stream, Flavour::Kinesis).unit_kb, 25.0);
        assert_eq!(cost(K::Queue, Flavour::Sqs).per_million_write, Some(1.2));
        assert_eq!(cost(K::Object, Flavour::Gcs), cost(K::Object, Flavour::S3));
    }
}
