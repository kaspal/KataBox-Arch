# Calibration

Every behaviour figure the simulator uses, with its value, unit and source (ADR 0005). The registry
(`crates/ka-core/src/registry.rs`) holds the figures; this file explains them. A test
(`crates/ka-core/tests/calibration.rs`) fails if a figure is missing here, if a value differs, or if a row is stale.

## How to read it
- **Row ids:**
  - `<kind>.<prop>` is a calibrated prop default; `@<flavour>` is a flavour's override.
  - `<kind>.servers_per_instance`, `service_ms`, `p99_ratio`, `queue_per_server` and `failover_s` are behaviour the
    card has no prop for.
  - `<kind>.cost.instance|lane|gb|million|million_write|unit_kb` are the prices: per instance, per shard or
    partition, per GB, per million reads, per million writes, and the billing unit size.
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
- **Prices** are list prices (us-east-1, on demand, read 2026-11-22) for the products cited in M7b, and teaching
  prices shaped like them elsewhere. No free tiers, discounts or data transfer. The judge compares designs with each
  other, not with a bill.
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
| `dns.query_ratio` | 0.01 | share of requests billed | source: needed (suggested: https://docs.aws.amazon.com/Route53/latest/DeveloperGuide/Welcome.html); billing only: clients and resolvers cache answers for the TTL, so about 1 request in 100 reaches the DNS service as a query |
| `dns.cost.million` | 0.4 | $/million queries | https://aws.amazon.com/route53/pricing/ — "$0.40 per million queries (first 1 billion queries per month)"; billed × `query_ratio`; the $0.50 hosted-zone fee is left out (list price, us-east-1, read 2026-11-22) |

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
| `serverless.concurrency_limit` | 1000 | in parallel | https://docs.aws.amazon.com/lambda/latest/dg/lambda-concurrency.html — "By default, Lambda provides your account with a total concurrency limit of 1,000 concurrent executions" per Region (checked 2026-11-22) |
| `serverless.concurrency_limit@cloud_run` | 8000 | in parallel | https://docs.cloud.google.com/run/docs/about-concurrency — default 80 concurrent requests per instance; https://docs.cloud.google.com/run/quotas — 100 instances per project and region by default; 100 × 80 = 8,000 (from search excerpts of both pages, not opened; checked 2026-11-22) |
| `serverless.cold_start_ms` | 250 | ms | https://aws.amazon.com/blogs/compute/operating-lambda-performance-optimization-part-1/ — AWS: cold starts vary "from under 100 ms to over 1 second" and occur in under 1% of production invocations; 250 ms is a small Node.js/Python function (an AWS blog, not reference docs) (checked 2026-11-22) |
| `serverless.cold_start_ms@cloud_run` | 500 | ms | source: needed (suggested: https://cloud.google.com/run/docs/about-instance-autoscaling); a container starts slower than a function runtime |
| `serverless.service_ms` | 20 | ms | source: needed (suggested: https://docs.aws.amazon.com/lambda/latest/dg/lambda-concurrency.html); a small handler |
| `serverless.p99_ratio` | 4 | × median | source: needed (suggested: https://docs.aws.amazon.com/lambda/latest/dg/lambda-concurrency.html); tail spread of the service time |
| `serverless.queue_per_server` | 0 | slots per server | source: needed (suggested: https://docs.aws.amazon.com/lambda/latest/dg/lambda-concurrency.html); invocations over the limit are throttled, not queued |
| `serverless.cost.million` | 0.5 | $/million requests | source: needed (suggested: https://aws.amazon.com/lambda/pricing/); the request price plus ~20 ms of duration at 512 MB |

### `sql`

| Id | Value | Unit | Source |
|---|---|---|---|
| `sql.concurrency` | 100 | in parallel per instance | https://www.postgresql.org/docs/current/runtime-config-connection.html — `max_connections`: "The default is typically 100 connections" (checked 2026-11-22) |
| `sql.concurrency@mysql` | 151 | in parallel per instance | https://dev.mysql.com/doc/refman/8.0/en/server-system-variables.html#sysvar_max_connections — Default Value 151 (variable table; corroborated by MySQL bug #99002 and the `max_connections_basic` test) (checked 2026-11-22) |
| `sql.service_ms` | 5 | ms | source: needed (suggested: https://www.postgresql.org/docs/current/runtime-config-connection.html); an indexed point read or a single-row write on a warm instance |
| `sql.replication_lag_ms` | 50 | ms | source: needed (suggested: https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/USER_ReadRepl.html); asynchronous streaming replication under moderate write load |
| `sql.failover_s` | 30 | s | https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/multi-az-db-clusters-concepts-failover.html — RDS Multi-AZ DB clusters: "Failover times are typically under 35 seconds"; single-standby instances take 60–120 s (https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/Concepts.MultiAZ.Failover.html). 30 s matches the cluster case (checked 2026-11-22) |
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
| `nosql.servers_per_instance@dynamodb` | 8 | in parallel per partition | https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/bp-partition-key-design.html — "a maximum capacity of 3,000 read units per second and 1,000 write units per second" per partition; a write takes 3 slots here (ADR 0006), and reads are modelled as strongly consistent, which is conservative (checked 2026-11-22) |
| `nosql.write_slots` | 1 | slots per write | source: needed (suggested: https://cassandra.apache.org/doc/latest/cassandra/architecture/storage-engine.html); a write costs a store about what a read does unless the flavour says otherwise |
| `nosql.write_slots@dynamodb` | 3 | slots per write | https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/bp-partition-key-design.html — a partition delivers "3,000 read units per second and 1,000 write units per second", so a write takes 3 read slots (checked 2026-11-22) |
| `nosql.cost.million` | 0.125 | $/million read units | https://aws.amazon.com/dynamodb/pricing/on-demand/ — "$0.125 per million reads" (one read unit per read of up to 4 KB); on demand, so no per-partition fee (list price, us-east-1, read 2026-11-22) |
| `nosql.cost.million_write` | 0.625 | $/million write units | https://aws.amazon.com/dynamodb/pricing/on-demand/ — "$0.6250 per million writes" (one write unit per 1 KB); the 3 write slots are a capacity rule, not a price (list price, us-east-1, read 2026-11-22) |
| `nosql.cost.lane@cassandra` | 250 | $/node-month (one per partition) | https://aws.amazon.com/ec2/pricing/on-demand/ — self-hosted on i4i.xlarge at $0.343/h × 730 (corroborated by https://calculator.holori.com/aws/ec2/i4i.xlarge/us-east-1); no per-request fee (list price, us-east-1, read 2026-11-22) |
| `nosql.cost.million@cassandra` | 0 | $/million read units | self-hosted: no per-request fee |
| `nosql.cost.million_write@cassandra` | 0 | $/million write units | self-hosted: no per-request fee |
| `nosql.cost.lane@mongodb` | 394 | $/shard-month (one per partition) | https://www.mongodb.com/pricing — Atlas M30 at $0.54/h × 730, taken as a 3-node replica set per shard (the Atlas default); no per-request fee (list price, us-east-1, read 2026-11-22) |
| `nosql.cost.million@mongodb` | 0 | $/million read units | Atlas dedicated clusters: no per-request fee |
| `nosql.cost.million_write@mongodb` | 0 | $/million write units | Atlas dedicated clusters: no per-request fee |

### `cache`

| Id | Value | Unit | Source |
|---|---|---|---|
| `cache.service_ms` | 0.01 | ms | https://redis.io/docs/latest/operate/oss_and_stack/management/optimization/benchmarks/ — redis-benchmark without pipelining: SET 180,180 req/s (p50 0.143 ms), 72,145 req/s over 100k random keys, one command thread; 0.01 ms ≈ 100k ops/s sits inside that range (checked 2026-11-22) |
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
| `object.servers_per_instance` | 1000 | in parallel per instance | https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance.html — "at least 3,500 PUT/COPY/POST/DELETE or 11,000 GET/HEAD requests per second per partitioned prefix"; 1,000 in parallel at 50 ms ≈ 20,000 GET/s ≈ two prefixes (checked 2026-11-22) |
| `object.p99_ratio` | 4 | × median | source: needed (suggested: https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance.html); tail spread of the service time |
| `object.queue_per_server` | 4 | slots per server | source: needed (suggested: https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance.html); backlog before the card drops requests |
| `object.cost.million` | 0.4 | $/million GETs | https://aws.amazon.com/s3/pricing/ — GET "$0.0004 per 1,000 requests"; GCS Standard Class B is the same (https://cloud.google.com/storage/pricing) (list price, us-east-1, read 2026-11-22) |
| `object.cost.million_write` | 5 | $/million PUTs | https://aws.amazon.com/s3/pricing/ — PUT $0.005 per 1,000 (corroborated by https://www.manageengine.com/cloudspend/articles/s3-pricing-guide.html); GCS Standard Class A is the same (list price, us-east-1, read 2026-11-22) |

### `search`

| Id | Value | Unit | Source |
|---|---|---|---|
| `search.service_ms` | 20 | ms | source: needed (suggested: https://www.elastic.co/guide/en/elasticsearch/reference/current/modules-threadpool.html); a simple match query on a warm shard |
| `search.servers_per_instance` | 13 | in parallel per shard | https://www.elastic.co/guide/en/elasticsearch/reference/current/modules-threadpool.html — search pool size "int((# of allocated processors * 3) / 2) + 1"; 13 at 8 processors (checked 2026-11-22) |
| `search.p99_ratio` | 4 | × median | source: needed (suggested: https://www.elastic.co/guide/en/elasticsearch/reference/current/modules-threadpool.html); tail spread of the service time |
| `search.queue_per_server` | 77 | slots per server | source: needed (suggested: https://www.elastic.co/guide/en/elasticsearch/reference/current/modules-threadpool.html); the search queue of 1,000 spread over 13 threads |
| `search.failover_s` | 10 | s | source: needed (suggested: https://www.elastic.co/guide/en/elasticsearch/reference/current/modules-threadpool.html); a replica shard is promoted when a node leaves |
| `search.cost.instance` | 250 | $/instance-month | source: needed (suggested: https://aws.amazon.com/opensearch-service/pricing/); teaching price, shaped like the list price |

### `queue`

| Id | Value | Unit | Source |
|---|---|---|---|
| `queue.visibility_timeout_s` | 30 | s | https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-visibility-timeout.html — "The default visibility timeout for a queue is 30 seconds" (checked 2026-11-22) |
| `queue.servers_per_instance` | 10000 | in parallel per instance | source: needed (suggested: https://www.rabbitmq.com/docs/confirms); brokers rarely limit a single design; consumers do |
| `queue.service_ms` | 2 | ms | source: needed (suggested: https://www.rabbitmq.com/docs/confirms); a RabbitMQ publish with confirms |
| `queue.p99_ratio` | 3 | × median | source: needed (suggested: https://www.rabbitmq.com/docs/confirms); tail spread of the service time |
| `queue.queue_per_server` | 4 | slots per server | source: needed (suggested: https://www.rabbitmq.com/docs/confirms); backlog before the card drops requests |
| `queue.service_ms@sqs` | 10 | ms | source: needed (suggested: https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/welcome.html); an SQS send over HTTPS |
| `queue.cost.million` | 0.4 | $/million retried receives | https://aws.amazon.com/sqs/pricing/ — every action is a request, $0.40 per million (https://aws.amazon.com/about-aws/whats-new/2016/11/amazon-sqs-introduces-fifo-queues-with-exactly-once-processing-and-lower-prices-for-standard-queues); a retry is one more receive; batching is not modelled (list price, us-east-1, read 2026-11-22) |
| `queue.cost.million_write` | 1.2 | $/million messages | https://aws.amazon.com/sqs/pricing/ — three requests per message (send, receive, delete) at $0.40 per million; no free tier (list price, us-east-1, read 2026-11-22) |
| `queue.cost.instance@rabbitmq` | 70 | $/node-month | https://aws.amazon.com/ec2/pricing/on-demand/ — self-hosted on m5.large at $0.096/h × 730; managed Amazon MQ mq.m5.large would be $0.288/h ≈ $210 (https://aws.amazon.com/amazon-mq/pricing/) (list price, us-east-1, read 2026-11-22) |
| `queue.cost.million@rabbitmq` | 0 | $/million retried receives | self-hosted: no per-request fee |
| `queue.cost.million_write@rabbitmq` | 0 | $/million messages | self-hosted: no per-request fee |

### `stream`

| Id | Value | Unit | Source |
|---|---|---|---|
| `stream.session_timeout_s` | 45 | s | https://kafka.apache.org/documentation/#consumerconfigs_session.timeout.ms — the consumer's `session.timeout.ms` defaults to 45000 since Kafka 3.0 (KIP-735); a dead consumer's partitions are reassigned after it (checked 2026-11-22, planning chat) |
| `stream.servers_per_instance` | 10000 | in parallel per instance | source: needed (suggested: https://kafka.apache.org/documentation/#producerconfigs); partitions, not the broker, limit consumers |
| `stream.service_ms` | 3 | ms | source: needed (suggested: https://kafka.apache.org/documentation/#producerconfigs); a Kafka produce with `acks=all` |
| `stream.p99_ratio` | 3 | × median | source: needed (suggested: https://kafka.apache.org/documentation/#producerconfigs); tail spread of the service time |
| `stream.queue_per_server` | 4 | slots per server | source: needed (suggested: https://kafka.apache.org/documentation/#producerconfigs); backlog before the card drops requests |
| `stream.service_ms@kinesis` | 15 | ms | source: needed (suggested: https://docs.aws.amazon.com/streams/latest/dev/service-sizes-and-limits.html); a Kinesis PutRecord |
| `stream.cost.instance` | 153 | $/broker-month | https://aws.amazon.com/msk/pricing/ — kafka.m5.large at $0.21/h × 730; no per-request fee, partitions free; storage ($0.10/GB-month) is not modelled (list price, us-east-1, read 2026-11-22) |
| `stream.cost.instance@kinesis` | 0 | $/broker-month | Kinesis bills shards, not brokers |
| `stream.cost.lane@kinesis` | 10.95 | $/shard-month (one per partition) | https://aws.amazon.com/kinesis/data-streams/pricing/ — $0.015 per shard-hour × 730 (list price, us-east-1, read 2026-11-22) |
| `stream.cost.million_write@kinesis` | 0.014 | $/million PUT payload units | https://aws.amazon.com/kinesis/data-streams/pricing/ — $0.014 per million PUT payload units of 25 KB (list price, us-east-1, read 2026-11-22) |
| `stream.cost.unit_kb@kinesis` | 25 | KB per billed unit | https://aws.amazon.com/kinesis/data-streams/pricing/ — a PUT payload unit is 25 KB |

### `pubsub`

| Id | Value | Unit | Source |
|---|---|---|---|
| `pubsub.service_ms` | 5 | ms | source: needed (suggested: https://cloud.google.com/pubsub/docs/overview); publish latency of a managed topic |
| `pubsub.servers_per_instance` | 10000 | in parallel per instance | source: needed (suggested: https://cloud.google.com/pubsub/docs/overview); managed fan-out scales with the topic |
| `pubsub.p99_ratio` | 3 | × median | source: needed (suggested: https://cloud.google.com/pubsub/docs/overview); tail spread of the service time |
| `pubsub.queue_per_server` | 4 | slots per server | source: needed (suggested: https://cloud.google.com/pubsub/docs/overview); backlog before the card drops requests |
| `pubsub.cost.million_write` | 0.5 | $/million publishes | https://aws.amazon.com/sns/faqs — "$0.50 per 1 million Amazon SNS Requests"; deliveries to SQS and Lambda are free (https://aws.amazon.com/sns/pricing/); HTTP deliveries ($0.60/M) are treated as free (list price, us-east-1, read 2026-11-22) |
| `pubsub.cost.million@google_pubsub` | 0.037 | $/million deliveries (1 KB) | https://cloud.google.com/pubsub/pricing — $40 per TiB with a 1 KB minimum per message; the first 10 GiB free is left out (list price, us-east-1, read 2026-11-22) |
| `pubsub.cost.million_write@google_pubsub` | 0.037 | $/million publishes (1 KB) | https://cloud.google.com/pubsub/pricing — as deliveries (list price, us-east-1, read 2026-11-22) |
| `pubsub.cost.unit_kb@google_pubsub` | 1 | KB per billed unit | https://cloud.google.com/pubsub/pricing — billed by volume, at least 1 KB per message |
| `pubsub.cost.instance@redis_pubsub` | 36 | $/node-month | self-hosted Redis: the cache node price (2 GB × `cache.cost.gb`); no per-message fee |
| `pubsub.cost.million_write@redis_pubsub` | 0 | $/million publishes | self-hosted: no per-message fee |

### `dlq`

| Id | Value | Unit | Source |
|---|---|---|---|
| `dlq.servers_per_instance` | 10000 | in parallel per instance | source: needed (suggested: https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-dead-letter-queues.html); as a queue |
| `dlq.service_ms` | 2 | ms | source: needed (suggested: https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-dead-letter-queues.html); as a queue |
| `dlq.p99_ratio` | 3 | × median | source: needed (suggested: https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-dead-letter-queues.html); tail spread of the service time |
| `dlq.queue_per_server` | 4 | slots per server | source: needed (suggested: https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-dead-letter-queues.html); backlog before the card drops requests |
| `dlq.cost.million` | 0.4 | $/million requests | https://aws.amazon.com/sqs/pricing/ — a dead-letter queue is an SQS queue (list price, us-east-1, read 2026-11-22) |
| `dlq.cost.million_write` | 1.2 | $/million messages | https://aws.amazon.com/sqs/pricing/ — as `queue.cost.million_write` (list price, us-east-1, read 2026-11-22) |

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
