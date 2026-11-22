# Calibration

Every behaviour figure the simulator uses, with its value, unit and source (ADR 0005). The registry
(`crates/ka-core/src/registry.rs`) holds the figures; this file explains them. A test
(`crates/ka-core/tests/calibration.rs`) fails if a figure is missing here, if a value differs, or if a row is stale.

## How to read it
- **Row ids:**
  - `<kind>.<prop>` is a calibrated prop default; `@<flavour>` is a flavour's override.
  - `<kind>.servers_per_instance`, `service_ms`, `p99_ratio`, `queue_per_server` and `failover_s` are behaviour the
    card has no prop for.
  - `<kind>.cost.instance|gb|million` is the teaching price.
- **What is calibrated, and what isn't:**
  - Calibrated: service times, concurrency, cold starts, replication lag, failover and visibility timeouts.
  - Not calibrated: design choices (instances, shards, partitions, memory, TTLs, retries, limits). Their defaults
    are only starting points.
- **Sources:**
  - `source: needed (suggested: <page>)` marks a conservative ballpark whose source has not been checked yet. It
    names the page to check.
  - The unattended M1–M3 run could not open web pages, so every row starts this way. Confirming a row means
    replacing its source with the citation and adding its id to `CITED` in the registry. The inspector then
    shows the figure as cited.
- **Prices** are a teaching model: consistent across designs and loosely shaped like public list prices, but
  never a quote. The judge compares designs with each other, not with a bill.
- **The model** the figures feed is described in ADR 0006:
  - a card has `servers = instances × concurrency`;
  - service times are lognormal, from the median and `p99_ratio`;
  - a queue holds `servers × queue_per_server` before it drops.

## Figures

### `dns`

| Id | Value | Unit | Source |
|---|---|---|---|
| `dns.servers_per_instance` | 10000 | in parallel per instance | source: needed (suggested: https://docs.aws.amazon.com/Route53/latest/DeveloperGuide/Welcome.html); a managed DNS service is never the bottleneck |
| `dns.service_ms` | 0.2 | ms | source: needed (suggested: https://docs.aws.amazon.com/Route53/latest/DeveloperGuide/Welcome.html); clients cache answers; only the resolver hit is paid |
| `dns.p99_ratio` | 3 | × median | source: needed (suggested: https://docs.aws.amazon.com/Route53/latest/DeveloperGuide/Welcome.html); tail spread of the service time |
| `dns.queue_per_server` | 4 | slots per server | source: needed (suggested: https://docs.aws.amazon.com/Route53/latest/DeveloperGuide/Welcome.html); backlog before the card drops requests |
| `dns.cost.million` | 0.4 | $/million requests | source: needed (suggested: https://aws.amazon.com/route53/pricing/); teaching price, shaped like the list price |

### `cdn`

| Id | Value | Unit | Source |
|---|---|---|---|
| `cdn.servers_per_instance` | 100000 | in parallel per instance | source: needed (suggested: https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/Introduction.html); edges are effectively unlimited for one design |
| `cdn.service_ms` | 2 | ms | source: needed (suggested: https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/Introduction.html); an edge cache hit near the user |
| `cdn.p99_ratio` | 4 | × median | source: needed (suggested: https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/Introduction.html); tail spread of the service time |
| `cdn.queue_per_server` | 4 | slots per server | source: needed (suggested: https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/Introduction.html); backlog before the card drops requests |
| `cdn.cost.million` | 0.75 | $/million requests | source: needed (suggested: https://aws.amazon.com/cloudfront/pricing/); teaching price, shaped like the list price |

### `waf`

| Id | Value | Unit | Source |
|---|---|---|---|
| `waf.service_ms` | 1 | ms | source: needed (suggested: https://docs.aws.amazon.com/waf/latest/developerguide/waf-chapter.html); rule evaluation per request |
| `waf.servers_per_instance` | 2000 | in parallel per instance | source: needed (suggested: https://docs.aws.amazon.com/waf/latest/developerguide/waf-chapter.html); requests one WAF instance inspects at once |
| `waf.p99_ratio` | 3 | × median | source: needed (suggested: https://docs.aws.amazon.com/waf/latest/developerguide/waf-chapter.html); tail spread of the service time |
| `waf.queue_per_server` | 8 | slots per server | source: needed (suggested: https://docs.aws.amazon.com/waf/latest/developerguide/waf-chapter.html); backlog before the card drops requests |
| `waf.cost.instance` | 20 | $/instance-month | source: needed (suggested: https://aws.amazon.com/waf/pricing/); teaching price, shaped like the list price |
| `waf.cost.million` | 0.6 | $/million requests | source: needed (suggested: https://aws.amazon.com/waf/pricing/); teaching price, shaped like the list price |

### `lb`

| Id | Value | Unit | Source |
|---|---|---|---|
| `lb.service_ms` | 0.5 | ms | source: needed (suggested: https://docs.aws.amazon.com/elasticloadbalancing/latest/application/introduction.html); proxying overhead of an L7 balancer |
| `lb.service_ms@aws_nlb` | 0.2 | ms | source: needed (suggested: https://docs.aws.amazon.com/elasticloadbalancing/latest/network/introduction.html); an L4 balancer does not parse HTTP |
| `lb.servers_per_instance` | 10000 | in parallel per instance | source: needed (suggested: https://docs.aws.amazon.com/elasticloadbalancing/latest/application/introduction.html); connections an instance proxies at once |
| `lb.p99_ratio` | 3 | × median | source: needed (suggested: https://docs.aws.amazon.com/elasticloadbalancing/latest/application/introduction.html); tail spread of the service time |
| `lb.queue_per_server` | 8 | slots per server | source: needed (suggested: https://docs.aws.amazon.com/elasticloadbalancing/latest/application/introduction.html); backlog before the card drops requests |
| `lb.cost.instance` | 25 | $/instance-month | source: needed (suggested: https://aws.amazon.com/elasticloadbalancing/pricing/); teaching price, shaped like the list price |

### `gateway`

| Id | Value | Unit | Source |
|---|---|---|---|
| `gateway.concurrency` | 256 | in parallel per instance | source: needed (suggested: https://docs.konghq.com/gateway/latest/); an Nginx-based proxy handles many connections per worker |
| `gateway.service_ms` | 2 | ms | source: needed (suggested: https://docs.konghq.com/gateway/latest/); auth, routing and plugins per request |
| `gateway.p99_ratio` | 4 | × median | source: needed (suggested: https://docs.konghq.com/gateway/latest/); tail spread of the service time |
| `gateway.queue_per_server` | 8 | slots per server | source: needed (suggested: https://docs.konghq.com/gateway/latest/); backlog before the card drops requests |
| `gateway.cost.instance` | 60 | $/instance-month | source: needed (suggested: https://aws.amazon.com/api-gateway/pricing/); teaching price, shaped like the list price |
| `gateway.cost.million` | 1 | $/million requests | source: needed (suggested: https://aws.amazon.com/api-gateway/pricing/); teaching price, shaped like the list price |

### `ratelimit`

| Id | Value | Unit | Source |
|---|---|---|---|
| `ratelimit.servers_per_instance` | 10000 | in parallel per instance | source: needed (suggested: https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/other_features/global_rate_limiting); a counter check is cheap |
| `ratelimit.service_ms` | 0.3 | ms | source: needed (suggested: https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/other_features/global_rate_limiting); a local token-bucket check |
| `ratelimit.p99_ratio` | 3 | × median | source: needed (suggested: https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/other_features/global_rate_limiting); tail spread of the service time |
| `ratelimit.queue_per_server` | 8 | slots per server | source: needed (suggested: https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/other_features/global_rate_limiting); backlog before the card drops requests |
| `ratelimit.cost.instance` | 30 | $/instance-month | source: needed (suggested: https://www.envoyproxy.io/docs/envoy/latest/intro/arch_overview/other_features/global_rate_limiting); teaching price, shaped like the list price |

### `app`

| Id | Value | Unit | Source |
|---|---|---|---|
| `app.concurrency` | 64 | in parallel per instance | source: needed (suggested: https://www.techempower.com/benchmarks/); requests a typical instance works on at once (threads and CPU together) |
| `app.concurrency@django` | 9 | in parallel per instance | source: needed (suggested: https://docs.gunicorn.org/en/stable/design.html#how-many-workers); gunicorn suggests (2 × cores) + 1 workers: 9 on 4 vCPUs |
| `app.service_ms` | 10 | ms | source: needed (suggested: https://www.techempower.com/benchmarks/); in-process work of a simple JSON endpoint, without downstream calls |
| `app.service_ms@go` | 5 | ms | source: needed (suggested: https://www.techempower.com/benchmarks/); a compiled, lighter runtime |
| `app.service_ms@django` | 20 | ms | source: needed (suggested: https://docs.gunicorn.org/en/stable/design.html#how-many-workers); a heavier framework on an interpreter |
| `app.p99_ratio` | 4 | × median | source: needed (suggested: https://www.techempower.com/benchmarks/); tail spread of the service time |
| `app.queue_per_server` | 8 | slots per server | source: needed (suggested: https://www.techempower.com/benchmarks/); backlog before the card drops requests |
| `app.cost.instance` | 70 | $/instance-month | source: needed (suggested: https://www.techempower.com/benchmarks/); teaching price, shaped like the list price |

### `micro`

| Id | Value | Unit | Source |
|---|---|---|---|
| `micro.concurrency` | 64 | in parallel per instance | source: needed (suggested: https://www.techempower.com/benchmarks/); as an app server |
| `micro.service_ms` | 10 | ms | source: needed (suggested: https://www.techempower.com/benchmarks/); as an app server |
| `micro.p99_ratio` | 4 | × median | source: needed (suggested: https://www.techempower.com/benchmarks/); tail spread of the service time |
| `micro.queue_per_server` | 8 | slots per server | source: needed (suggested: https://www.techempower.com/benchmarks/); backlog before the card drops requests |
| `micro.cost.instance` | 70 | $/instance-month | source: needed (suggested: https://www.techempower.com/benchmarks/); teaching price, shaped like the list price |

### `worker`

| Id | Value | Unit | Source |
|---|---|---|---|
| `worker.concurrency` | 8 | in parallel per instance | source: needed (suggested: https://docs.celeryq.dev/en/stable/userguide/workers.html#concurrency); Celery defaults to the CPU count: 8 on 8 vCPUs |
| `worker.concurrency@sidekiq` | 5 | in parallel per instance | source: needed (suggested: https://github.com/sidekiq/sidekiq/wiki/Advanced-Options#concurrency); Sidekiq 7 defaults to 5 threads |
| `worker.service_ms` | 100 | ms | source: needed (suggested: https://docs.celeryq.dev/en/stable/userguide/workers.html#concurrency); a typical background job (rendering, a few writes) |
| `worker.p99_ratio` | 4 | × median | source: needed (suggested: https://docs.celeryq.dev/en/stable/userguide/workers.html#concurrency); tail spread of the service time |
| `worker.queue_per_server` | 8 | slots per server | source: needed (suggested: https://docs.celeryq.dev/en/stable/userguide/workers.html#concurrency); backlog before the card drops requests |
| `worker.cost.instance` | 70 | $/instance-month | source: needed (suggested: https://docs.celeryq.dev/en/stable/userguide/workers.html#concurrency); teaching price, shaped like the list price |

### `serverless`

| Id | Value | Unit | Source |
|---|---|---|---|
| `serverless.concurrency_limit` | 1000 | in parallel | source: needed (suggested: https://docs.aws.amazon.com/lambda/latest/dg/lambda-concurrency.html); the default per-account concurrency quota of AWS Lambda |
| `serverless.concurrency_limit@cloud_run` | 8000 | in parallel | source: needed (suggested: https://cloud.google.com/run/docs/about-instance-autoscaling); 100 instances × 80 concurrent requests, the default maximums |
| `serverless.cold_start_ms` | 250 | ms | source: needed (suggested: https://docs.aws.amazon.com/lambda/latest/dg/lambda-runtime-environment.html); a small Node.js or Python function |
| `serverless.cold_start_ms@cloud_run` | 500 | ms | source: needed (suggested: https://cloud.google.com/run/docs/about-instance-autoscaling); a container starts slower than a function runtime |
| `serverless.service_ms` | 20 | ms | source: needed (suggested: https://docs.aws.amazon.com/lambda/latest/dg/lambda-concurrency.html); a small handler |
| `serverless.p99_ratio` | 4 | × median | source: needed (suggested: https://docs.aws.amazon.com/lambda/latest/dg/lambda-concurrency.html); tail spread of the service time |
| `serverless.queue_per_server` | 0 | slots per server | source: needed (suggested: https://docs.aws.amazon.com/lambda/latest/dg/lambda-concurrency.html); invocations over the limit are throttled, not queued |
| `serverless.cost.million` | 0.5 | $/million requests | source: needed (suggested: https://aws.amazon.com/lambda/pricing/); the request price plus ~20 ms of duration at 512 MB |

### `sql`

| Id | Value | Unit | Source |
|---|---|---|---|
| `sql.concurrency` | 100 | in parallel per instance | source: needed (suggested: https://www.postgresql.org/docs/current/runtime-config-connection.html); PostgreSQL `max_connections` defaults to 100 |
| `sql.concurrency@mysql` | 151 | in parallel per instance | source: needed (suggested: https://dev.mysql.com/doc/refman/8.0/en/server-system-variables.html#sysvar_max_connections); MySQL `max_connections` defaults to 151 |
| `sql.service_ms` | 5 | ms | source: needed (suggested: https://www.postgresql.org/docs/current/runtime-config-connection.html); an indexed point read or a single-row write on a warm instance |
| `sql.replication_lag_ms` | 50 | ms | source: needed (suggested: https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/USER_ReadRepl.html); asynchronous streaming replication under moderate write load |
| `sql.failover_s` | 30 | s | source: needed (suggested: https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/Concepts.MultiAZSingleStandby.html); managed failover to a standby; vendors quote one to two minutes, shortened to fit 60 s tests |
| `sql.p99_ratio` | 5 | × median | source: needed (suggested: https://www.postgresql.org/docs/current/runtime-config-connection.html); tail spread of the service time |
| `sql.queue_per_server` | 4 | slots per server | source: needed (suggested: https://www.postgresql.org/docs/current/runtime-config-connection.html); backlog before the card drops requests |
| `sql.cost.instance` | 350 | $/instance-month | source: needed (suggested: https://aws.amazon.com/rds/postgresql/pricing/); teaching price, shaped like the list price |

### `nosql`

| Id | Value | Unit | Source |
|---|---|---|---|
| `nosql.service_ms` | 3 | ms | source: needed (suggested: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/bp-partition-key-design.html); a single-item read or write |
| `nosql.servers_per_instance` | 32 | in parallel per partition | source: needed (suggested: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/bp-partition-key-design.html); per partition (or token range) of a self-run cluster |
| `nosql.p99_ratio` | 4 | × median | source: needed (suggested: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/bp-partition-key-design.html); tail spread of the service time |
| `nosql.queue_per_server` | 4 | slots per server | source: needed (suggested: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/bp-partition-key-design.html); backlog before the card drops requests |
| `nosql.failover_s` | 5 | s | source: needed (suggested: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/bp-partition-key-design.html); replicas already hold the data; the client retries another node |
| `nosql.servers_per_instance@dynamodb` | 8 | in parallel per partition | source: needed (suggested: https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/bp-partition-key-design.html); 8 in parallel at ~3 ms ≈ 2,700 ops/s, near the documented 3,000 reads/s per partition |
| `nosql.cost.instance` | 120 | $/instance-month | source: needed (suggested: https://aws.amazon.com/dynamodb/pricing/); teaching price, shaped like the list price |
| `nosql.cost.million` | 0.5 | $/million requests | source: needed (suggested: https://aws.amazon.com/dynamodb/pricing/); teaching price, shaped like the list price |

### `cache`

| Id | Value | Unit | Source |
|---|---|---|---|
| `cache.service_ms` | 0.01 | ms | source: needed (suggested: https://redis.io/docs/latest/operate/oss_and_stack/management/optimization/benchmarks/); one Redis event loop serves about 100,000 simple GET/SET per second |
| `cache.servers_per_instance` | 1 | in parallel per instance | source: needed (suggested: https://redis.io/docs/latest/operate/oss_and_stack/management/optimization/benchmarks/); Redis runs commands on one thread |
| `cache.p99_ratio` | 3 | × median | source: needed (suggested: https://redis.io/docs/latest/operate/oss_and_stack/management/optimization/benchmarks/); tail spread of the service time |
| `cache.queue_per_server` | 512 | slots per server | source: needed (suggested: https://redis.io/docs/latest/operate/oss_and_stack/management/optimization/benchmarks/); clients pipeline many commands on one connection |
| `cache.failover_s` | 15 | s | source: needed (suggested: https://docs.aws.amazon.com/AmazonElastiCache/latest/dg/AutoFailover.html); replica promotion in a managed Redis |
| `cache.servers_per_instance@memcached` | 4 | in parallel per instance | source: needed (suggested: https://github.com/memcached/memcached/wiki/ConfiguringServer); memcached starts 4 worker threads (`-t 4`) |
| `cache.cost.gb` | 18 | $/GB-month | source: needed (suggested: https://aws.amazon.com/elasticache/pricing/); teaching price, shaped like the list price |

### `object`

| Id | Value | Unit | Source |
|---|---|---|---|
| `object.service_ms` | 50 | ms | source: needed (suggested: https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance.html); time to first byte of a small object |
| `object.servers_per_instance` | 1000 | in parallel per instance | source: needed (suggested: https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance.html); ≈ 20,000 GET/s at 50 ms: a few prefixes at the documented 5,500 GET/s each |
| `object.p99_ratio` | 4 | × median | source: needed (suggested: https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance.html); tail spread of the service time |
| `object.queue_per_server` | 4 | slots per server | source: needed (suggested: https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance.html); backlog before the card drops requests |
| `object.cost.million` | 1 | $/million requests | source: needed (suggested: https://aws.amazon.com/s3/pricing/); a read-heavy blend of GET (~$0.4/M) and PUT (~$5/M) list prices |

### `search`

| Id | Value | Unit | Source |
|---|---|---|---|
| `search.service_ms` | 20 | ms | source: needed (suggested: https://www.elastic.co/guide/en/elasticsearch/reference/current/modules-threadpool.html); a simple match query on a warm shard |
| `search.servers_per_instance` | 13 | in parallel per shard | source: needed (suggested: https://www.elastic.co/guide/en/elasticsearch/reference/current/modules-threadpool.html); the search thread pool on 8 vCPUs: int(3 × 8 / 2) + 1 |
| `search.p99_ratio` | 4 | × median | source: needed (suggested: https://www.elastic.co/guide/en/elasticsearch/reference/current/modules-threadpool.html); tail spread of the service time |
| `search.queue_per_server` | 77 | slots per server | source: needed (suggested: https://www.elastic.co/guide/en/elasticsearch/reference/current/modules-threadpool.html); the search queue of 1,000 spread over 13 threads |
| `search.failover_s` | 10 | s | source: needed (suggested: https://www.elastic.co/guide/en/elasticsearch/reference/current/modules-threadpool.html); a replica shard is promoted when a node leaves |
| `search.cost.instance` | 250 | $/instance-month | source: needed (suggested: https://aws.amazon.com/opensearch-service/pricing/); teaching price, shaped like the list price |

### `queue`

| Id | Value | Unit | Source |
|---|---|---|---|
| `queue.visibility_timeout_s` | 30 | s | source: needed (suggested: https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-visibility-timeout.html); the SQS default |
| `queue.servers_per_instance` | 10000 | in parallel per instance | source: needed (suggested: https://www.rabbitmq.com/docs/confirms); brokers rarely limit a single design; consumers do |
| `queue.service_ms` | 2 | ms | source: needed (suggested: https://www.rabbitmq.com/docs/confirms); a RabbitMQ publish with confirms |
| `queue.p99_ratio` | 3 | × median | source: needed (suggested: https://www.rabbitmq.com/docs/confirms); tail spread of the service time |
| `queue.queue_per_server` | 4 | slots per server | source: needed (suggested: https://www.rabbitmq.com/docs/confirms); backlog before the card drops requests |
| `queue.service_ms@sqs` | 10 | ms | source: needed (suggested: https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/welcome.html); an SQS send over HTTPS |
| `queue.cost.instance` | 40 | $/instance-month | source: needed (suggested: https://aws.amazon.com/sqs/pricing/); teaching price, shaped like the list price |
| `queue.cost.million` | 0.4 | $/million requests | source: needed (suggested: https://aws.amazon.com/sqs/pricing/); teaching price, shaped like the list price |

### `stream`

| Id | Value | Unit | Source |
|---|---|---|---|
| `stream.servers_per_instance` | 10000 | in parallel per instance | source: needed (suggested: https://kafka.apache.org/documentation/#producerconfigs); partitions, not the broker, limit consumers |
| `stream.service_ms` | 3 | ms | source: needed (suggested: https://kafka.apache.org/documentation/#producerconfigs); a Kafka produce with `acks=all` |
| `stream.p99_ratio` | 3 | × median | source: needed (suggested: https://kafka.apache.org/documentation/#producerconfigs); tail spread of the service time |
| `stream.queue_per_server` | 4 | slots per server | source: needed (suggested: https://kafka.apache.org/documentation/#producerconfigs); backlog before the card drops requests |
| `stream.service_ms@kinesis` | 15 | ms | source: needed (suggested: https://docs.aws.amazon.com/streams/latest/dev/service-sizes-and-limits.html); a Kinesis PutRecord |
| `stream.cost.instance` | 15 | $/instance-month | source: needed (suggested: https://aws.amazon.com/kinesis/data-streams/pricing/); teaching price, shaped like the list price |
| `stream.cost.million` | 0.2 | $/million requests | source: needed (suggested: https://aws.amazon.com/kinesis/data-streams/pricing/); teaching price, shaped like the list price |

### `pubsub`

| Id | Value | Unit | Source |
|---|---|---|---|
| `pubsub.service_ms` | 5 | ms | source: needed (suggested: https://cloud.google.com/pubsub/docs/overview); publish latency of a managed topic |
| `pubsub.servers_per_instance` | 10000 | in parallel per instance | source: needed (suggested: https://cloud.google.com/pubsub/docs/overview); managed fan-out scales with the topic |
| `pubsub.p99_ratio` | 3 | × median | source: needed (suggested: https://cloud.google.com/pubsub/docs/overview); tail spread of the service time |
| `pubsub.queue_per_server` | 4 | slots per server | source: needed (suggested: https://cloud.google.com/pubsub/docs/overview); backlog before the card drops requests |
| `pubsub.cost.million` | 0.5 | $/million requests | source: needed (suggested: https://cloud.google.com/pubsub/pricing); teaching price, shaped like the list price |

### `dlq`

| Id | Value | Unit | Source |
|---|---|---|---|
| `dlq.servers_per_instance` | 10000 | in parallel per instance | source: needed (suggested: https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-dead-letter-queues.html); as a queue |
| `dlq.service_ms` | 2 | ms | source: needed (suggested: https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-dead-letter-queues.html); as a queue |
| `dlq.p99_ratio` | 3 | × median | source: needed (suggested: https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-dead-letter-queues.html); tail spread of the service time |
| `dlq.queue_per_server` | 4 | slots per server | source: needed (suggested: https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-dead-letter-queues.html); backlog before the card drops requests |
| `dlq.cost.million` | 0.4 | $/million requests | source: needed (suggested: https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-dead-letter-queues.html); teaching price, shaped like the list price |

### `scheduler`

| Id | Value | Unit | Source |
|---|---|---|---|
| `scheduler.cost.instance` | 20 | $/instance-month | source: needed (suggested: https://airflow.apache.org/docs/); teaching price, shaped like the list price |

### `metrics`

| Id | Value | Unit | Source |
|---|---|---|---|
| `metrics.cost.instance` | 100 | $/instance-month | source: needed (suggested: https://prometheus.io/docs/introduction/overview/); teaching price, shaped like the list price |

### `logs`

| Id | Value | Unit | Source |
|---|---|---|---|
| `logs.cost.instance` | 150 | $/instance-month | source: needed (suggested: https://grafana.com/docs/loki/latest/); teaching price, shaped like the list price |

### `tracing`

| Id | Value | Unit | Source |
|---|---|---|---|
| `tracing.cost.instance` | 80 | $/instance-month | source: needed (suggested: https://www.jaegertracing.io/docs/); teaching price, shaped like the list price |
