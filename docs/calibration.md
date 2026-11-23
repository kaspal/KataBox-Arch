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
- **Sources:** every row has one of three kinds of source, and the registry's lists say which (a test keeps them in
  step). The inspector shows the kind next to the figure.
  - **Cited:** the cell gives the URL and the quote or figure the value comes from. The id is in `CITED`.
    "via <site>" means the vendor's own price table didn't render, so a third-party mirror was read;
    "third-party measurement" means an independent measurement site, not the vendor.
  - **Rule of thumb:** the cell starts `rule of thumb` and says what the convention is. The id is in
    `RULE_OF_THUMB`.
  - **Model:** the cell starts `model: <why>`. These are simulator knobs no vendor publishes: tail multipliers,
    backlog slots, the concurrency of managed services that are effectively unlimited, and mirrors of another card.
    A context page, where one helps, follows as `context: <URL>`. The id is in `MODEL`, and the app shows it as a
    modelling choice.
  - Anything else reads `source: needed`: a conservative ballpark whose source is not confirmed. Where a source was
    found but says something different from the value, the cell reads `ballpark; the source says <figure>` with the
    URL, and the value is kept until the change has been checked against the problems' verdicts.
- **Prices** are public list prices (AWS us-east-1, GCP us-central1, Azure East US, or the vendor's standard list;
  on demand; read 2026-11-22/23). No free tiers, discounts or data transfer. The judge compares designs with each
  other, not with a bill. Model prices (`infer`) change often: re-check model prices each release.
- **The model** the figures feed is described in ADR 0006:
  - a card has `servers = instances × concurrency`;
  - service times are lognormal, from the median and `p99_ratio`;
  - a queue holds `servers × queue_per_server` before it drops.

## Figures

### `dns`

| Id | Value | Unit | Source |
|---|---|---|---|
| `dns.servers_per_instance` | 10000 | in parallel per instance | model: anycast managed DNS; no per-instance figure; means "never the bottleneck" |
| `dns.service_ms` | 0.2 | ms | model: AWS publishes no server-side time; DNSPerf measures network round trips (tens of ms), not processing; clients cache answers; only the resolver hit is paid; context: https://www.dnsperf.com/dns-provider/route53 |
| `dns.p99_ratio` | 3 | × median | model: tail multiplier |
| `dns.queue_per_server` | 4 | slots per server | model: backlog slots |
| `dns.query_ratio` | 0.01 | share of requests billed | model: depends on the TTL and resolver caching; the price it scales is "$0.40 per million queries (first 1 billion queries per month)"; billing only: clients and resolvers cache answers for the TTL, so about 1 request in 100 reaches the DNS service as a query; context: https://aws.amazon.com/route53/pricing/ |
| `dns.cost.million` | 0.4 | $/million queries | https://aws.amazon.com/route53/pricing/ — "$0.40 per million queries (first 1 billion queries per month)"; billed × `query_ratio`; the $0.50 hosted-zone fee is left out (list price, us-east-1, read 2026-11-22) |

### `cdn`

| Id | Value | Unit | Source |
|---|---|---|---|
| `cdn.servers_per_instance` | 100000 | in parallel per instance | model: managed edge, effectively unlimited; edges are effectively unlimited for one design |
| `cdn.service_ms` | 2 | ms | model: no published processing time for a hit; an edge cache hit near the user; context: https://aws.amazon.com/premiumsupport/knowledge-center/cloudfront-troubleshoot-latency/ |
| `cdn.p99_ratio` | 4 | × median | model: tail multiplier |
| `cdn.queue_per_server` | 4 | slots per server | model: backlog slots |
| `cdn.cost.million` | 0.75 | $/million requests | source: needed; ballpark; the source says US HTTPS requests $0.0100 per 10,000 = $1.00/M (0.75 was the HTTP price) — https://aws.amazon.com/cloudfront/pricing/pay-as-you-go |
| `cdn.cost.million@akamai` | 1.2 | $/million requests | model: Akamai publishes no list prices; context: https://blog.blazingcdn.com/en-us/what-is-the-price-per-gb-of-akamai-cdn |

### `waf`

| Id | Value | Unit | Source |
|---|---|---|---|
| `waf.service_ms` | 1 | ms | https://aws.github.io/aws-security-services-best-practices/guides/waf/operationalizing/docs/ — "at most adding single-digit millisecond latency"; rule evaluation per request |
| `waf.servers_per_instance` | 2000 | in parallel per instance | model: managed; no published concurrency; requests one WAF instance inspects at once |
| `waf.p99_ratio` | 3 | × median | model: tail multiplier |
| `waf.queue_per_server` | 8 | slots per server | model: backlog slots |
| `waf.cost.instance` | 20 | $/instance-month | https://aws.amazon.com/waf/pricing/ — Web ACL $5.00/month + $1.00 per rule per month (1 ACL + 15 rules = $20) |
| `waf.cost.million` | 0.6 | $/million requests | https://aws.amazon.com/waf/pricing/ — "$0.60/million" requests |

### `lb`

| Id | Value | Unit | Source |
|---|---|---|---|
| `lb.service_ms` | 0.5 | ms | model: no published ALB added latency; proxying overhead of an L7 balancer; context: https://docs.aws.amazon.com/wellarchitected/latest/framework/perf_networking_load_balancing_distribute_traffic.html |
| `lb.service_ms@aws_nlb` | 0.2 | ms | model: only "ultra low latency", no figure; an L4 balancer does not parse HTTP; context: https://aws.amazon.com/blogs/aws/new-network-load-balancer-effortless-scaling-to-millions-of-requests-per-second |
| `lb.servers_per_instance` | 10000 | in parallel per instance | model: managed, scales automatically; connections an instance proxies at once |
| `lb.p99_ratio` | 3 | × median | model: tail multiplier |
| `lb.queue_per_server` | 8 | slots per server | model: backlog slots |
| `lb.cost.instance` | 25 | $/instance-month | https://aws.amazon.com/elasticloadbalancing/pricing/ — ALB $0.0225/h + $0.008 per LCU-hour: 730 h + ~1.5 LCU ≈ $25/month |
| `lb.service_ms@global_accelerator` | 0.2 | ms | model: no per-request overhead published; context: https://aws.amazon.com/global-accelerator/faqs |
| `lb.cost.instance@envoy_lb` | 70 | $/instance-month | https://calculator.holori.com/aws/ec2/m5.large/us-east-1 — m5.large $0.096/h ≈ $70.08/month (via holori) |
| `lb.cost.instance@traefik` | 70 | $/instance-month | https://calculator.holori.com/aws/ec2/m5.large/us-east-1 — m5.large ≈ $70.08/month |
| `lb.cost.instance@global_accelerator` | 18 | $/instance-month | https://aws.amazon.com/global-accelerator/pricing/ — $0.025 per accelerator-hour ≈ $18/month |
| `lb.cost.million@global_accelerator` | 1.5 | $/million requests | https://aws.amazon.com/global-accelerator/pricing/ — DT-Premium ≈ $0.015/GB (NA→EU example); assumes ~100 KB per request: 1M × 100 KB ≈ $1.50 |

### `gateway`

| Id | Value | Unit | Source |
|---|---|---|---|
| `gateway.concurrency` | 256 | in parallel per instance | model: per-instance knob; Kong publishes no concurrency figure; an Nginx-based proxy handles many connections per worker |
| `gateway.service_ms` | 2 | ms | https://developer.konghq.com/gateway/performance/benchmarks/ — Kong 3.16, c5.4xlarge, no plugins: p95 2.89 ms, p99 4.5 ms; with rate limit + key auth: p95 4.41 ms; auth, routing and plugins per request |
| `gateway.p99_ratio` | 4 | × median | model: tail multiplier (Kong's p99/p95 ≈ 1.6–1.9×, no median published) |
| `gateway.queue_per_server` | 8 | slots per server | model: backlog slots |
| `gateway.cost.instance` | 60 | $/instance-month | https://calculator.holori.com/aws/ec2/c5.large/us-east-1 — Self-hosted c5.large "$0.0850/hr ~$62.05/month" (list price via holori) |
| `gateway.cost.million` | 1 | $/million requests | https://aws.amazon.com/api-gateway/pricing/ — HTTP API $1.00/million (REST API $3.50/million) |
| `gateway.service_ms@apollo_router` | 3 | ms | https://apollographql.com/blog/apollo-router-our-graphql-federation-runtime-in-rust — "adds less than 10ms of latency to each operation" |
| `gateway.cost.instance@azure_apim` | 686 | $/instance-month | https://www.activepieces.com/blog/azure-api-management-pricing-tier-costs-in-2026 — Standard tier "$686.72" a month (via activepieces) |
| `gateway.cost.million@azure_apim` | 0 | $/million requests | billed another way (see the other rows for this flavour) |
| `gateway.cost.instance@litellm` | 70 | $/instance-month | https://calculator.holori.com/aws/ec2/m5.large/us-east-1 — ≈ $70.08/month |
| `gateway.cost.million@litellm` | 0 | $/million requests | billed another way (see the other rows for this flavour) |
| `gateway.cost.instance@apollo_router` | 70 | $/instance-month | https://calculator.holori.com/aws/ec2/m5.large/us-east-1 — ≈ $70.08/month |
| `gateway.cost.million@apollo_router` | 0 | $/million requests | billed another way (see the other rows for this flavour) |

### `ratelimit`

| Id | Value | Unit | Source |
|---|---|---|---|
| `ratelimit.servers_per_instance` | 10000 | in parallel per instance | model: concurrency knob; a counter check is cheap |
| `ratelimit.service_ms` | 0.3 | ms | https://aws.amazon.com/blogs/database/optimize-redis-client-performance-for-amazon-elasticache/ — "typical latency for one command is 0.2 millisecond"; plus the Envoy hop; a local token-bucket check |
| `ratelimit.p99_ratio` | 3 | × median | model: tail multiplier |
| `ratelimit.queue_per_server` | 8 | slots per server | model: backlog slots |
| `ratelimit.cost.instance` | 30 | $/instance-month | https://calculator.holori.com/aws?region=us-east-1 — t3.medium $0.0416/h ≈ $30.37/month (list price via holori) |

### `app`

| Id | Value | Unit | Source |
|---|---|---|---|
| `app.concurrency` | 64 | in parallel per instance | model: defaults vary by runtime (Puma 3–5 threads, gunicorn 2×cores+1, async runtimes far more); requests a typical instance works on at once (threads and CPU together); context: https://docs.gunicorn.org/en/21.0.1/design.html |
| `app.concurrency@django` | 9 | in parallel per instance | source: needed; gunicorn suggests (2 × cores) + 1 workers: 9 on 4 vCPUs; ballpark; the source says "(2 x $num_cores) + 1" on the 2-vCPU m5.large the cost row prices — https://docs.gunicorn.org/en/21.0.1/design.html |
| `app.service_ms` | 10 | ms | model: benchmark-shaped ballpark; TechEmpower publishes rps, not per-request time; in-process work of a simple JSON endpoint, without downstream calls; context: https://www.techempower.com/benchmarks/ |
| `app.service_ms@go` | 5 | ms | model: relative to the base value; a compiled, lighter runtime; context: https://www.techempower.com/benchmarks/ |
| `app.service_ms@django` | 20 | ms | model: relative to the base value; a heavier framework on an interpreter; context: https://www.techempower.com/benchmarks/ |
| `app.p99_ratio` | 4 | × median | model: tail multiplier |
| `app.queue_per_server` | 8 | slots per server | model: backlog slots |
| `app.cost.instance` | 70 | $/instance-month | https://calculator.holori.com/aws/ec2/m5.large/us-east-1 — m5.large "$0.0960/hr … ~$70.08/month" (list price via holori) |
| `app.concurrency@rails` | 5 | in parallel per instance | source: needed; ballpark; the source says Default Puma threads "set to 3" — https://guides.rubyonrails.org/tuning_performance_for_deployment.html |
| `app.service_ms@aspnet_core` | 6 | ms | model: no vendor per-request figure |
| `app.service_ms@rails` | 25 | ms | model: no vendor per-request figure |

### `micro`

| Id | Value | Unit | Source |
|---|---|---|---|
| `micro.concurrency` | 64 | in parallel per instance | model: as `app.concurrency` |
| `micro.service_ms` | 10 | ms | model: as `app.service_ms` |
| `micro.p99_ratio` | 4 | × median | model: tail multiplier |
| `micro.queue_per_server` | 8 | slots per server | model: backlog slots |
| `micro.cost.instance` | 70 | $/instance-month | https://calculator.holori.com/aws/ec2/m5.large/us-east-1 — m5.large ≈ $70.08/month |

### `worker`

| Id | Value | Unit | Source |
|---|---|---|---|
| `worker.concurrency` | 8 | in parallel per instance | https://manpages.ubuntu.com/manpages/noble/man1/celery-worker.1.html — default "is the number of CPUs" (2 on the m5.large the cost row prices); 8 is a configured value (`--concurrency`) for I/O-bound jobs |
| `worker.concurrency@sidekiq` | 5 | in parallel per instance | https://sources.debian.org/src/ruby-sidekiq/7.3.2%2Bdfsg-1/lib/sidekiq/config.rb — Sidekiq 7 default `concurrency: 5` ("The default capsule spins up 5 Threads", 7.0 upgrade notes) |
| `worker.service_ms` | 100 | ms | model: no vendor figure for "a typical background job"; a typical background job (rendering, a few writes) |
| `worker.p99_ratio` | 4 | × median | model: tail multiplier |
| `worker.queue_per_server` | 8 | slots per server | model: backlog slots |
| `worker.cost.instance` | 70 | $/instance-month | https://calculator.holori.com/aws/ec2/m5.large/us-east-1 — m5.large ≈ $70.08/month |
| `worker.concurrency@bullmq` | 10 | in parallel per instance | https://docs.bullmq.io/guide/workers/concurrency — the default is 1; 10 is a configured value |
| `worker.concurrency@temporal_worker` | 100 | in parallel per instance | https://docs.temporal.io/develop/ruby/activities/basics — Default tuner "defaults to 100 activities at a time" (other SDKs differ) |
| `worker.concurrency@ffmpeg` | 2 | in parallel per instance | model: sizing assumption for a 2-vCPU instance |
| `worker.concurrency@mediaconvert` | 1000 | in parallel per instance | source: needed; ballpark; the source says Concurrent jobs per on-demand queue, us-east-1: 500 (default 200) — https://docs.aws.amazon.com/general/latest/gr/mediaconvert.html |
| `worker.service_ms@ffmpeg` | 10000 | ms | model: depends on the input and preset |
| `worker.service_ms@mediaconvert` | 10000 | ms | model: no published job duration |
| `worker.cost.instance@mediaconvert` | 0 | $/instance-month | billed another way (see the other rows for this flavour) |
| `worker.cost.million@mediaconvert` | 7500 | $/million requests | https://aws.amazon.com/mediaconvert/pricing/ — $0.0075 per SD output minute → $7,500 per million one-minute jobs |

### `serverless`

| Id | Value | Unit | Source |
|---|---|---|---|
| `serverless.concurrency_limit` | 1000 | in parallel | https://docs.aws.amazon.com/lambda/latest/dg/lambda-concurrency.html — "By default, Lambda provides your account with a total concurrency limit of 1,000 concurrent executions" per Region (checked 2026-11-22) |
| `serverless.concurrency_limit@cloud_run` | 8000 | in parallel | https://docs.cloud.google.com/run/docs/about-concurrency — default 80 concurrent requests per instance; https://docs.cloud.google.com/run/quotas — 100 instances per project and region by default; 100 × 80 = 8,000 (from search excerpts of both pages, not opened; checked 2026-11-22) |
| `serverless.cold_start_ms` | 250 | ms | https://aws.amazon.com/blogs/compute/operating-lambda-performance-optimization-part-1/ — AWS: cold starts vary "from under 100 ms to over 1 second" and occur in under 1% of production invocations; 250 ms is a small Node.js/Python function (an AWS blog, not reference docs) (checked 2026-11-22) |
| `serverless.cold_start_ms@cloud_run` | 500 | ms | model: Google gives no number; third-party measurements range from tens of ms to seconds, depending on the image; a container starts slower than a function runtime; context: https://cloud.google.com/run/docs/tips |
| `serverless.service_ms` | 20 | ms | model: workload-dependent; a small handler |
| `serverless.p99_ratio` | 4 | × median | model: tail multiplier |
| `serverless.queue_per_server` | 0 | slots per server | https://docs.aws.amazon.com/lambda/latest/dg/lambda-concurrency.html — Synchronous invocations over the limit get a throttling error (429); "Lambda does not queue throttled requests" (web-functions scaling page); invocations over the limit are throttled, not queued |
| `serverless.cost.million` | 0.5 | $/million requests | source: needed; the request price plus ~20 ms of duration at 512 MB; ballpark; the source says $0.20 per 1M requests + 0.512 GB × 0.020 s × $0.0000166667/GB-s × 10⁶ ≈ $0.17 (x86) — https://aws.amazon.com/lambda/pricing/ |
| `serverless.cold_start_ms@cloudflare_workers` | 5 | ms | https://blog.cloudflare.com/cloud-computing-without-containers/ — "Isolates start in 5 milliseconds" |
| `serverless.service_ms@cloudflare_workers` | 10 | ms | model: no published service time; context: https://developers.cloudflare.com/workers/platform/pricing/ |
| `serverless.cost.million@cloudflare_workers` | 0.3 | $/million requests | https://developers.cloudflare.com/workers/platform/pricing/ — "+$0.30 per additional million" requests (+ CPU ms) |
| `serverless.cost.million@azure_functions` | 0.4 | $/million requests | https://www.activepieces.com/blog/azure-functions-pricing-all-plans-compared-2026 — Flex Consumption "$0.40 per million" (via activepieces) |

### `sql`

| Id | Value | Unit | Source |
|---|---|---|---|
| `sql.concurrency` | 100 | in parallel per instance | https://www.postgresql.org/docs/current/runtime-config-connection.html — `max_connections`: "The default is typically 100 connections" (checked 2026-11-22) |
| `sql.concurrency@mysql` | 151 | in parallel per instance | https://dev.mysql.com/doc/refman/8.0/en/server-system-variables.html#sysvar_max_connections — Default Value 151 (variable table; corroborated by MySQL bug #99002 and the `max_connections_basic` test) (checked 2026-11-22) |
| `sql.service_ms` | 5 | ms | model: no vendor figure; a client round trip plus an indexed execution; an indexed point read or a single-row write on a warm instance |
| `sql.replication_lag_ms` | 50 | ms | https://docs.aws.amazon.com/AmazonRDS/latest/AuroraUserGuide/Aurora.Replication.html — Aurora figure: "usually much less than 100 milliseconds"; the RDS PostgreSQL docs give no ms figure; asynchronous streaming replication under moderate write load |
| `sql.failover_s` | 30 | s | https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/multi-az-db-clusters-concepts-failover.html — RDS Multi-AZ DB clusters: "Failover times are typically under 35 seconds"; single-standby instances take 60–120 s (https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/Concepts.MultiAZ.Failover.html). 30 s matches the cluster case (checked 2026-11-22) |
| `sql.p99_ratio` | 5 | × median | model: tail multiplier |
| `sql.queue_per_server` | 4 | slots per server | model: backlog slots |
| `sql.cost.instance` | 350 | $/instance-month | source: needed; ballpark; the source says db.m5.xlarge PostgreSQL, us-east-1, on-demand ≈ $260/month (list price via bytebase) — https://www.bytebase.com/dbcost/rds/instance/db.m5.xlarge/ |
| `sql.max_connections` | 1700 | connections | https://aws.amazon.com/premiumsupport/knowledge-center/rds-mysql-max-connections/ — LEAST(DBInstanceClassMemory/9531392, 5000); 16 GiB gives ≤ 1802, less the memory RDS reserves |
| `sql.max_connections@mysql` | 1365 | connections | source: needed; RDS MySQL default DBInstanceClassMemory/12582880 on db.m5.xlarge; ballpark; the source says DBInstanceClassMemory/12582880 ≤ 1365, with the same reserve — https://aws.amazon.com/premiumsupport/knowledge-center/rds-mysql-max-connections/ |
| `sql.service_ms@cockroachdb` | 12 | ms | source: needed; ballpark; the source says "single-row writes in 2 ms within a single availability zone"; +Raft across AZs — https://cockroachlabs.com/docs/v24.2/performance |
| `sql.service_ms@spanner` | 12 | ms | source: needed; ballpark; the source says "single-digit millisecond latency" (regional) — https://cloud.google.com/blog/products/databases/benchmarking-spanner-for-key-value-workloads |
| `sql.service_ms@postgis` | 10 | ms | model: workload-dependent; context: indexed ST_DWithin ~2–26 ms |
| `sql.service_ms@citus` | 5.5 | ms | model: coordinator hop, no published figure |
| `sql.replication_lag_ms@aurora_postgresql` | 20 | ms | https://docs.aws.amazon.com/AmazonRDS/latest/AuroraUserGuide/Aurora.Replication.html — "usually much less than 100 milliseconds"; "10s of milliseconds" (AWS knowledge center) |
| `sql.cost.instance@cockroachdb` | 438 | $/instance-month | source: needed; ballpark; the source says $0.162/vCPU-h (Mission Critical) × 4 × 730 — https://cockroachlabs.com/pricing |
| `sql.cost.instance@aurora_postgresql` | 300 | $/instance-month | source: needed; ballpark; the source says db.r6g.large Aurora PostgreSQL "$190/month" (via bytebase) — https://www.bytebase.com/dbcost/aurora/instance/db.r6g.large/ |
| `sql.cost.instance@spanner` | 657 | $/instance-month | https://cloud.google.com/spanner/docs/cuds — "$0.9 per node per hour" (us-central1, Standard) |

### `nosql`

| Id | Value | Unit | Source |
|---|---|---|---|
| `nosql.service_ms` | 3 | ms | https://aws.amazon.com/blogs/database/reduce-latency-and-cost-in-read-heavy-applications-using-amazon-dynamodb-accelerator/ — "approximately 2.4 milliseconds" per item retrieval; DynamoDB: "single-digit millisecond performance"; a single-item read or write |
| `nosql.servers_per_instance` | 32 | in parallel per partition | model: per-partition parallelism; per partition (or token range) of a self-run cluster |
| `nosql.p99_ratio` | 4 | × median | model: tail multiplier |
| `nosql.queue_per_server` | 4 | slots per server | model: backlog slots |
| `nosql.failover_s` | 5 | s | model: client timeout plus a retry to another replica; no vendor figure; replicas already hold the data; the client retries another node |
| `nosql.servers_per_instance@dynamodb` | 8 | in parallel per partition | https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/bp-partition-key-design.html — "a maximum capacity of 3,000 read units per second and 1,000 write units per second" per partition; a write takes 3 slots here (ADR 0006), and reads are modelled as strongly consistent, which is conservative (checked 2026-11-22) |
| `nosql.write_slots` | 1 | slots per write | https://cassandra.apache.org/doc/latest/cassandra/architecture/storage-engine.html — "a write path free of read lookups and bottlenecks"; a write costs a store about what a read does unless the flavour says otherwise |
| `nosql.write_slots@dynamodb` | 3 | slots per write | https://docs.aws.amazon.com/amazondynamodb/latest/developerguide/bp-partition-key-design.html — a partition delivers "3,000 read units per second and 1,000 write units per second", so a write takes 3 read slots (checked 2026-11-22) |
| `nosql.cost.million` | 0.125 | $/million read units | https://aws.amazon.com/dynamodb/pricing/on-demand/ — "$0.125 per million reads" (one read unit per read of up to 4 KB); on demand, so no per-partition fee (list price, us-east-1, read 2026-11-22) |
| `nosql.cost.million_write` | 0.625 | $/million write units | https://aws.amazon.com/dynamodb/pricing/on-demand/ — "$0.6250 per million writes" (one write unit per 1 KB); the 3 write slots are a capacity rule, not a price (list price, us-east-1, read 2026-11-22) |
| `nosql.cost.lane@cassandra` | 250 | $/node-month (one per partition) | https://aws.amazon.com/ec2/pricing/on-demand/ — self-hosted on i4i.xlarge at $0.343/h × 730 (corroborated by https://calculator.holori.com/aws/ec2/i4i.xlarge/us-east-1); no per-request fee (list price, us-east-1, read 2026-11-22) |
| `nosql.cost.million@cassandra` | 0 | $/million read units | self-hosted: no per-request fee |
| `nosql.cost.million_write@cassandra` | 0 | $/million write units | self-hosted: no per-request fee |
| `nosql.cost.lane@mongodb` | 394 | $/shard-month (one per partition) | https://www.mongodb.com/pricing — Atlas M30 at $0.54/h × 730, taken as a 3-node replica set per shard (the Atlas default); no per-request fee (list price, us-east-1, read 2026-11-22) |
| `nosql.cost.million@mongodb` | 0 | $/million read units | Atlas dedicated clusters: no per-request fee |
| `nosql.cost.million_write@mongodb` | 0 | $/million write units | Atlas dedicated clusters: no per-request fee |
| `nosql.servers_per_instance@scylladb` | 96 | in parallel per instance | https://www.scylladb.com/2021/08/24/scylladb-vs-cassandra-performance-results/ — "2x-5x better throughput than Cassandra 4.0" |
| `nosql.cost.lane@scylladb` | 250 | $/shard-month | https://calculator.holori.com/aws/ec2/i4i.xlarge/us-east-1 — i4i.xlarge $0.343/h ≈ $250.39 |
| `nosql.cost.million@scylladb` | 0 | $/million requests | billed another way (see the other rows for this flavour) |
| `nosql.cost.million_write@scylladb` | 0 | $/million writes | billed another way (see the other rows for this flavour) |
| `nosql.cost.lane@bigtable` | 474 | $/shard-month | https://cloud.google.com/bigtable/pricing — "$0.65 / 1 hour" per node (us-central1) |
| `nosql.cost.million@bigtable` | 0 | $/million requests | billed another way (see the other rows for this flavour) |
| `nosql.cost.million_write@bigtable` | 0 | $/million writes | billed another way (see the other rows for this flavour) |
| `nosql.cost.million@cosmos_db` | 0.25 | $/million requests | https://learn.microsoft.com/en-us/azure/cosmos-db/key-value-store-cost — 1 KB read = 1 RU; serverless $0.25 per million RUs |
| `nosql.cost.million_write@cosmos_db` | 1.25 | $/million writes | https://learn.microsoft.com/en-us/azure/cosmos-db/key-value-store-cost — 1 KB write = 5 RUs × $0.25/M |

### `cache`

| Id | Value | Unit | Source |
|---|---|---|---|
| `cache.service_ms` | 0.01 | ms | https://redis.io/docs/latest/operate/oss_and_stack/management/optimization/benchmarks/ — redis-benchmark without pipelining: SET 180,180 req/s (p50 0.143 ms), 72,145 req/s over 100k random keys, one command thread; 0.01 ms ≈ 100k ops/s sits inside that range (checked 2026-11-22) |
| `cache.servers_per_instance` | 1 | in parallel per instance | https://redis.io/docs/latest/operate/oss_and_stack/management/optimization/benchmarks/ — "Redis is, mostly, a single-threaded server from the POV of commands execution" |
| `cache.p99_ratio` | 3 | × median | model: tail multiplier |
| `cache.queue_per_server` | 512 | slots per server | model: pipelining backlog; clients pipeline many commands on one connection |
| `cache.failover_s` | 15 | s | source: needed; replica promotion in a managed Redis; ballpark; the source says "Writes can resume as soon as the promotion process is complete, typically just a few seconds" — https://docs.aws.amazon.com/AmazonElastiCache/latest/dg/AutoFailover.html |
| `cache.servers_per_instance@memcached` | 4 | in parallel per instance | https://manpages.debian.org/unstable/memcached/memcached.1.en.html — "-t … Number of threads to use … The default is 4" |
| `cache.cost.gb` | 18 | $/GB-month | source: needed; ballpark; the source says cache.r7g.large 13.07 GiB at $0.219/h → $159.87/month ÷ 13.07 ≈ $12.2/GB-month (list price via vantage) — https://instances.vantage.sh/aws/elasticache/cache.r7g.large |
| `cache.servers_per_instance@dragonfly` | 8 | in parallel per instance | https://dragonflydb.io/blog/2024-new-year-new-number — One thread per shard and core; 8 = an 8-vCPU node |
| `cache.servers_per_instance@hazelcast` | 8 | in parallel per instance | model: partition threads scale with cores |

### `object`

| Id | Value | Unit | Source |
|---|---|---|---|
| `object.service_ms` | 50 | ms | source: needed; time to first byte of a small object; ballpark; the source says "small object latencies … of roughly 100–200 milliseconds" — https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance.html |
| `object.servers_per_instance` | 1000 | in parallel per instance | https://docs.aws.amazon.com/AmazonS3/latest/userguide/optimizing-performance.html — "at least 3,500 PUT/COPY/POST/DELETE or 11,000 GET/HEAD requests per second per partitioned prefix"; 1,000 in parallel at 50 ms ≈ 20,000 GET/s ≈ two prefixes (checked 2026-11-22) |
| `object.p99_ratio` | 4 | × median | model: tail multiplier |
| `object.queue_per_server` | 4 | slots per server | model: backlog slots |
| `object.cost.million` | 0.4 | $/million GETs | https://aws.amazon.com/s3/pricing/ — GET "$0.0004 per 1,000 requests"; GCS Standard Class B is the same (https://cloud.google.com/storage/pricing) (list price, us-east-1, read 2026-11-22) |
| `object.cost.million_write` | 5 | $/million PUTs | https://aws.amazon.com/s3/pricing/ — PUT $0.005 per 1,000 (corroborated by https://www.manageengine.com/cloudspend/articles/s3-pricing-guide.html); GCS Standard Class A is the same (list price, us-east-1, read 2026-11-22) |
| `object.cost.million@azure_blob` | 0.44 | $/million requests | https://learn.microsoft.com/en-us/azure/storage/blobs/blob-storage-estimate-costs — Read "$0.00000044" each = $0.44/M |
| `object.cost.million_write@azure_blob` | 5.5 | $/million writes | https://learn.microsoft.com/en-us/azure/storage/blobs/blob-storage-estimate-costs — Hot write "$0.055" per 10,000 = $5.50/M |
| `object.cost.million@cloudflare_r2` | 0.36 | $/million requests | https://www.cloudflare.com/rates/ — Class B "$0.36 / million requests" |
| `object.cost.million_write@cloudflare_r2` | 4.5 | $/million writes | https://www.cloudflare.com/rates/ — Class A "$4.50 / million requests" |
| `object.cost.instance@minio` | 70 | $/instance-month | model: self-hosted node; no list price (m5.large context) |
| `object.cost.million@minio` | 0 | $/million requests | billed another way (see the other rows for this flavour) |
| `object.cost.million_write@minio` | 0 | $/million writes | billed another way (see the other rows for this flavour) |

### `search`

| Id | Value | Unit | Source |
|---|---|---|---|
| `search.service_ms` | 20 | ms | model: no vendor figure for a simple match query; a simple match query on a warm shard |
| `search.servers_per_instance` | 13 | in parallel per shard | https://www.elastic.co/guide/en/elasticsearch/reference/current/modules-threadpool.html — search pool size "int((# of allocated processors * 3) / 2) + 1"; 13 at 8 processors (checked 2026-11-22) |
| `search.p99_ratio` | 4 | × median | model: tail multiplier |
| `search.queue_per_server` | 77 | slots per server | source: needed; the search queue of 1,000 spread over 13 threads; ballpark; the source says Elasticsearch 9: search `queue_size` is "1000 * size" (per thread). The old 1000 ÷ 13 ≈ 77 only held for 8.x — https://www.elastic.co/docs/reference/elasticsearch/configuration-reference/thread-pool-settings |
| `search.failover_s` | 10 | s | source: needed; a replica shard is promoted when a node leaves; ballpark; the source says An unresponsive node is removed after 3 failed 10 s checks, then replicas are promoted (a clean disconnect is immediate); model the unresponsive case — https://www.elastic.co/guide/en/elasticsearch/reference/8.7/cluster-fault-detection.html |
| `search.cost.instance` | 250 | $/instance-month | https://bigdataboutique.com/blog/opensearch-and-elasticsearch-pricing-guide — r6g.xlarge.search, 4 vCPUs, 32 GiB, $0.335/h ≈ $245/month (list price via bigdataboutique) |
| `search.service_ms@meilisearch` | 5 | ms | https://github.com/meilisearch/meilisearch — "less than 50 milliseconds" |
| `search.cost.instance@meilisearch` | 150 | $/instance-month | source: needed; ballpark; the source says Cloud Pro "$300/mo" (via aiproductivity) — https://aiproductivity.ai/pricing/meilisearch/ |

### `queue`

| Id | Value | Unit | Source |
|---|---|---|---|
| `queue.visibility_timeout_s` | 30 | s | https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/sqs-visibility-timeout.html — "The default visibility timeout for a queue is 30 seconds" (checked 2026-11-22) |
| `queue.servers_per_instance` | 10000 | in parallel per instance | model: effectively unlimited; brokers rarely limit a single design; consumers do |
| `queue.service_ms` | 2 | ms | https://www.cloudamqp.com/blog/publishing-throughput-asynchronous-vs-synchronous.html — Synchronous publish with confirms: "400-500 messages a second" per publisher ≈ 2–2.5 ms; a RabbitMQ publish with confirms |
| `queue.p99_ratio` | 3 | × median | model: tail multiplier |
| `queue.queue_per_server` | 4 | slots per server | model: backlog slots |
| `queue.service_ms@sqs` | 10 | ms | source: needed; an SQS send over HTTPS; ballpark; the source says "if the latency from an Amazon EC2-based client to Amazon SQS in the same region averages 20 ms" — https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/throughput.html |
| `queue.cost.million` | 0.4 | $/million retried receives | https://aws.amazon.com/sqs/pricing/ — every action is a request, $0.40 per million (https://aws.amazon.com/about-aws/whats-new/2016/11/amazon-sqs-introduces-fifo-queues-with-exactly-once-processing-and-lower-prices-for-standard-queues); a retry is one more receive; batching is not modelled (list price, us-east-1, read 2026-11-22) |
| `queue.cost.million_write` | 1.2 | $/million messages | https://aws.amazon.com/sqs/pricing/ — three requests per message (send, receive, delete) at $0.40 per million; no free tier (list price, us-east-1, read 2026-11-22) |
| `queue.cost.instance@rabbitmq` | 70 | $/node-month | https://aws.amazon.com/ec2/pricing/on-demand/ — self-hosted on m5.large at $0.096/h × 730; managed Amazon MQ mq.m5.large would be $0.288/h ≈ $210 (https://aws.amazon.com/amazon-mq/pricing/) (list price, us-east-1, read 2026-11-22) |
| `queue.cost.million@rabbitmq` | 0 | $/million retried receives | self-hosted: no per-request fee |
| `queue.cost.million_write@rabbitmq` | 0 | $/million messages | self-hosted: no per-request fee |
| `queue.servers_per_instance@sqs_fifo` | 300 | in parallel per instance | https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/troubleshooting-fifo-throttling-issues.html — "300 transactions per second, per API action" |
| `queue.service_ms@sqs_fifo` | 20 | ms | https://docs.aws.amazon.com/AWSSimpleQueueService/latest/SQSDeveloperGuide/throughput.html — "averages 20 ms" example |
| `queue.cost.million@azure_service_bus` | 0.8 | $/million requests | https://discuss.particular.net/t/budgeting-for-azure-service-bus/319/2 — "$0.80 per million operations" (13M–100M) (via Particular) |
| `queue.cost.million_write@azure_service_bus` | 2.4 | $/million writes | https://discuss.particular.net/t/budgeting-for-azure-service-bus/319/2 — send + receive + complete = 3 operations |
| `queue.cost.instance@amazon_mq` | 210 | $/instance-month | https://aws.amazon.com/amazon-mq/pricing/ — mq.m5.large $0.288/h |
| `queue.cost.million@amazon_mq` | 0 | $/million requests | billed another way (see the other rows for this flavour) |
| `queue.cost.million_write@amazon_mq` | 0 | $/million writes | billed another way (see the other rows for this flavour) |
| `queue.cost.million@sqs_fifo` | 0.5 | $/million requests | https://aws.amazon.com/about-aws/whats-new/2016/11/amazon-sqs-introduces-fifo-queues-with-exactly-once-processing-and-lower-prices-for-standard-queues — "$0.50 per million API requests" |
| `queue.cost.million_write@sqs_fifo` | 1.5 | $/million writes | https://aws.amazon.com/about-aws/whats-new/2016/11/amazon-sqs-introduces-fifo-queues-with-exactly-once-processing-and-lower-prices-for-standard-queues — 3 requests per message |

### `stream`

| Id | Value | Unit | Source |
|---|---|---|---|
| `stream.session_timeout_s` | 45 | s | https://kafka.apache.org/documentation/#consumerconfigs_session.timeout.ms — the consumer's `session.timeout.ms` defaults to 45000 since Kafka 3.0 (KIP-735); a dead consumer's partitions are reassigned after it (checked 2026-11-22, planning chat) |
| `stream.servers_per_instance` | 10000 | in parallel per instance | model: partitions, not brokers, limit consumers |
| `stream.service_ms` | 3 | ms | https://www.confluent.io/blog/kafka-fastest-messaging-system/ — p99 end-to-end "5 ms (200 MB/s load)"; the throughput test used `acks=all`; a Kafka produce with `acks=all` |
| `stream.p99_ratio` | 3 | × median | model: tail multiplier |
| `stream.queue_per_server` | 4 | slots per server | model: backlog slots |
| `stream.service_ms@kinesis` | 15 | ms | source: needed; a Kinesis PutRecord; ballpark; the source says "P50 manages to stay right around the 35 ms mark" (third-party measurement; AWS publishes no figure) — https://brandur.org/kinesis-in-production |
| `stream.cost.instance` | 153 | $/broker-month | https://aws.amazon.com/msk/pricing/ — kafka.m5.large at $0.21/h × 730; no per-request fee, partitions free; storage ($0.10/GB-month) is not modelled (list price, us-east-1, read 2026-11-22) |
| `stream.cost.instance@kinesis` | 0 | $/broker-month | Kinesis bills shards, not brokers |
| `stream.cost.lane@kinesis` | 10.95 | $/shard-month (one per partition) | https://aws.amazon.com/kinesis/data-streams/pricing/ — $0.015 per shard-hour × 730 (list price, us-east-1, read 2026-11-22) |
| `stream.cost.million_write@kinesis` | 0.014 | $/million PUT payload units | https://aws.amazon.com/kinesis/data-streams/pricing/ — $0.014 per million PUT payload units of 25 KB (list price, us-east-1, read 2026-11-22) |
| `stream.cost.unit_kb@kinesis` | 25 | KB per billed unit | https://aws.amazon.com/kinesis/data-streams/pricing/ — a PUT payload unit is 25 KB |
| `stream.service_ms@redpanda` | 1 | ms | source: needed; ballpark; the source says acks=all average 2.09 ms — https://computingforgeeks.com/kafka-vs-redpanda-benchmarks/ |
| `stream.cost.instance@azure_event_hubs` | 0 | $/instance-month | billed another way (see the other rows for this flavour) |
| `stream.cost.lane@azure_event_hubs` | 21.9 | $/shard-month | https://www.pump.co/blog/azure-event-hub — "$0.03/hour per throughput unit" (via pump.co) |
| `stream.cost.million_write@azure_event_hubs` | 0.028 | $/million writes | https://www.pump.co/blog/azure-event-hub — "$0.028 per million ingress events" |

### `pubsub`

| Id | Value | Unit | Source |
|---|---|---|---|
| `pubsub.service_ms` | 5 | ms | model: no vendor publishes a publish latency for SNS or Pub/Sub; SQS's 20 ms would be an HTTPS-API analogue; publish latency of a managed topic |
| `pubsub.servers_per_instance` | 10000 | in parallel per instance | model: effectively unlimited; managed fan-out scales with the topic |
| `pubsub.p99_ratio` | 3 | × median | model: tail multiplier |
| `pubsub.queue_per_server` | 4 | slots per server | model: backlog slots |
| `pubsub.cost.million_write` | 0.5 | $/million publishes | https://aws.amazon.com/sns/faqs — "$0.50 per 1 million Amazon SNS Requests"; deliveries to SQS and Lambda are free (https://aws.amazon.com/sns/pricing/); HTTP deliveries ($0.60/M) are treated as free (list price, us-east-1, read 2026-11-22) |
| `pubsub.cost.million@google_pubsub` | 0.037 | $/million deliveries (1 KB) | https://cloud.google.com/pubsub/pricing — $40 per TiB with a 1 KB minimum per message; the first 10 GiB free is left out (list price, us-east-1, read 2026-11-22) |
| `pubsub.cost.million_write@google_pubsub` | 0.037 | $/million publishes (1 KB) | https://cloud.google.com/pubsub/pricing — as deliveries (list price, us-east-1, read 2026-11-22) |
| `pubsub.cost.unit_kb@google_pubsub` | 1 | KB per billed unit | https://cloud.google.com/pubsub/pricing — billed by volume, at least 1 KB per message |
| `pubsub.cost.instance@redis_pubsub` | 36 | $/node-month | self-hosted Redis: the cache node price (2 GB × `cache.cost.gb`); no per-message fee |
| `pubsub.cost.million_write@redis_pubsub` | 0 | $/million publishes | self-hosted: no per-message fee |
| `pubsub.service_ms@nats` | 0.5 | ms | https://www.nats.io/about — "sub-millisecond latency" |
| `pubsub.cost.instance@nats` | 70 | $/instance-month | model: self-hosted node (m5.large context) |
| `pubsub.cost.million_write@nats` | 0 | $/million writes | billed another way (see the other rows for this flavour) |
| `pubsub.cost.million_write@azure_web_pubsub` | 1 | $/million writes | https://www.infoworld.com/article/3617314/build-real-time-communication-applications-with-azure-web-pubsub.html — "$1.61 per unit per day and $1 per million messages" (a preview-era price) |
| `pubsub.cost.million_write@eventbridge` | 1 | $/million writes | https://aws.amazon.com/eventbridge/pricing — "$1.00/M" custom events |

### `dlq`

| Id | Value | Unit | Source |
|---|---|---|---|
| `dlq.servers_per_instance` | 10000 | in parallel per instance | model: mirrors its queue |
| `dlq.service_ms` | 2 | ms | model: mirrors its queue |
| `dlq.p99_ratio` | 3 | × median | model: mirrors its queue |
| `dlq.queue_per_server` | 4 | slots per server | model: mirrors its queue |
| `dlq.cost.million` | 0.4 | $/million requests | https://aws.amazon.com/sqs/pricing/ — a dead-letter queue is an SQS queue (list price, us-east-1, read 2026-11-22) |
| `dlq.cost.million_write` | 1.2 | $/million messages | https://aws.amazon.com/sqs/pricing/ — as `queue.cost.million_write` (list price, us-east-1, read 2026-11-22) |

### `scheduler`

| Id | Value | Unit | Source |
|---|---|---|---|
| `scheduler.cost.instance` | 20 | $/instance-month | https://calculator.holori.com/aws/ec2/t3.small/us-east-1 — t3.small $0.0208/h ≈ $15.18/month + storage ≈ $20 (context: Amazon MWAA mw1.small $0.49/h) |
| `scheduler.cost.instance@temporal` | 200 | $/instance-month | source: needed; ballpark; the source says "Starting at $50 per million actions" (Temporal Cloud usage price, no minimum) — https://temporal.io/pricing |
| `scheduler.cost.instance@dagster` | 100 | $/instance-month | https://dagster.io/blog/dagster-cloud-pricing-philosophy — "Starter is $100/month plus $0.035 per credit" |
| `scheduler.cost.instance@k8s_cronjob` | 0 | $/instance-month | billed another way (see the other rows for this flavour) |

### `metrics`

| Id | Value | Unit | Source |
|---|---|---|---|
| `metrics.cost.instance` | 100 | $/instance-month | https://aws.amazon.com/prometheus/pricing/ — Amazon Managed Prometheus' worked example totals $81.75/month; ≈ $100 for a small fleet |

### `logs`

| Id | Value | Unit | Source |
|---|---|---|---|
| `logs.cost.instance` | 150 | $/instance-month | https://aws.amazon.com/cloudwatch/pricing/ — Logs ingest $0.50/GB; $150 ≈ 300 GB/month |

### `tracing`

| Id | Value | Unit | Source |
|---|---|---|---|
| `tracing.cost.instance` | 80 | $/instance-month | https://aws.amazon.com/cloudwatch/pricing/ — X-Ray $5 per million traces recorded; $80 ≈ 16M traces/month |

### `ws`

| Id | Value | Unit | Source |
|---|---|---|---|
| `ws.max_connections` | 100000 | connections per instance | https://dev.to/chengyixu/building-a-production-ready-websocket-server-with-nodejs-scaling-to-100k-connections-25mk — "roughly 50,000 to 100,000" per Node process; one gateway process holding idle sockets |
| `ws.max_connections@phoenix` | 1000000 | connections per instance | https://phoenixframework.org/blog/the-road-to-2-million-websocket-connections — 2M on one 40-core box |
| `ws.conn_memory_kb` | 20 | KB per connection | https://websockets.readthedocs.io/en/14.2/topics/memory.html — 14 KiB without compression, 64 with; socket buffers and per-connection state |
| `ws.msgs_per_s` | 50000 | messages/s per instance | model: conservative per instance; server pushes one instance sends |
| `ws.accept_per_s` | 2000 | new connections/s per instance | https://websocket.org/guides/connection-limits/ — "1,000-3,000 per second per core" TLS handshakes; TLS handshakes plus auth one instance accepts |
| `ws.service_ms` | 0.5 | ms | model: event-loop cost |
| `ws.servers_per_instance` | 10000 | in parallel per instance | model: effectively unlimited; the card's own limits (connections, pools) bind first |
| `ws.p99_ratio` | 3 | × median | model: tail multiplier of the service time |
| `ws.queue_per_server` | 8 | waiting per server | model: backlog slots before drops |
| `ws.cost.instance` | 70 | $/instance-month | https://calculator.holori.com/aws/ec2/m5.large/us-east-1 — ≈ $70.08 |
| `ws.cost.instance@aws_apigw_ws` | 0 | $/instance-month | billed another way (see the other rows for this flavour) |
| `ws.cost.meter.conn_minutes@aws_apigw_ws` | 0.25 | $/million connection-minutes | https://aws.amazon.com/api-gateway/pricing/ — $0.25 per million connection-minutes |
| `ws.cost.meter.messages@aws_apigw_ws` | 1 | $/million messages | https://aws.amazon.com/api-gateway/pricing/ — $1.00 per million messages (32 KB increments) |

### `idp`

| Id | Value | Unit | Source |
|---|---|---|---|
| `idp.login_ms` | 150 | ms | model: hosted login, no canonical figure; a password login with token issue |
| `idp.verify_ms` | 5 | ms | model: local JWT check; a token introspection call |
| `idp.logins_per_s` | 100 | logins/s | https://auth0.com/docs/troubleshoot/customer-support/operational-policies/rate-limit-policy — Authentication API (production) "100/second"; the tenant's login rate limit |
| `idp.servers_per_instance` | 256 | in parallel per instance | model: abstraction |
| `idp.service_ms` | 5 | ms | model: abstraction |
| `idp.p99_ratio` | 4 | × median | model: tail multiplier of the service time |
| `idp.queue_per_server` | 8 | waiting per server | model: backlog slots before drops |
| `idp.cost.million` | 5 | $/million requests | source: needed; ballpark; the source says Cognito Essentials $0.015/MAU; at an assumed 5 logins per MAU a month, $3,000 per million logins — https://builder.aws.com/content/3KlTXfJhkIuYFkyPkM2P16amtXC/sign-in-for-10000-users-at-dollar0-with-amazon-cognito |
| `idp.cost.instance@keycloak` | 70 | $/instance-month | https://calculator.holori.com/aws/ec2/m5.large/us-east-1 — ≈ $70.08 |
| `idp.cost.million@keycloak` | 0 | $/million requests | billed another way (see the other rows for this flavour) |

### `external`

| Id | Value | Unit | Source |
|---|---|---|---|
| `external.cost.million@twilio` | 7900 | $/million requests | source: needed; ballpark; the source says "starts at $0.0083" per outbound SMS — https://www.twilio.com/en-us/sms/pricing/us |
| `external.cost.million@sendgrid` | 900 | $/million requests | https://sendgrid.com/en-us/pricing — Pro $89.95 for 100,000 emails |

### `infer`

| Id | Value | Unit | Source |
|---|---|---|---|
| `infer.ttft_ms` | 600 | ms to first token | source: needed; time to first token of a hosted model; ballpark; the source says Sonnet 5.5 low effort ≈ 1.05–1.18 s to first token (third-party measurement) — https://artificialanalysis.ai/models/claude-sonnet-5-5 |
| `infer.ttft_ms@openai_api` | 500 | ms to first token | source: needed; time to first token of a hosted model; ballpark; the source says "TTFT of 0.89s" (third-party measurement) — https://artificialanalysis.ai/models/gpt-6-sol-non-reasoning |
| `infer.ttft_ms@gemini_api` | 500 | ms to first token | source: needed; ballpark; the source says Gemini 3.1 Pro always reasons; its measured 14.8 s includes thinking; 1000 would match the other non-reasoning first-token figures (0.9–1.1 s) (third-party measurement) — https://artificialanalysis.ai/models/gemini-3-1-pro-preview |
| `infer.ttft_ms@vllm` | 200 | ms to first token | https://docs.vllm.ai/en/v0.13.0/benchmarking/cli/; https://www.truefoundry.com/blog/vllm-benchmark — Median 74 ms (light load); p50 388 ms at 16 users on one A10; time to first token of a hosted model |
| `infer.ttft_ms@triton` | 5 | ms to first token | model: one forward pass of a small classifier |
| `infer.tokens_per_s` | 80 | tokens/s per request | source: needed; output speed of one stream; ballpark; the source says Sonnet 5.5: 96–103 t/s (third-party measurement) — https://artificialanalysis.ai/leaderboards/models |
| `infer.tokens_per_s@openai_api` | 90 | tokens/s per request | source: needed; output speed of one stream; ballpark; the source says GPT-6.2 Sol 62–67 t/s (third-party measurement) — https://artificialanalysis.ai/models/gpt-6-2-sol-high |
| `infer.tokens_per_s@gemini_api` | 120 | tokens/s per request | https://artificialanalysis.ai/models/gemini-3-1-pro-preview/providers — 122.5–127.5 t/s (third-party measurement); output speed of one stream |
| `infer.prefill_tps` | 5000 | prompt tokens/s | model: depends on the GPU, model and batch; prompt processing speed |
| `infer.rpm` | 4000 | requests/min | https://platform.claude.com/docs/en/api/rate-limits — Start tier, Sonnet 5.5: 4,000 RPM; a hosted account tier's request limit |
| `infer.tpm` | 400000 | tokens/min | https://platform.claude.com/docs/en/api/rate-limits — Start tier: OTPM 400,000 (ITPM 2,000,000); the simulator's tpm is the output limit; a hosted account tier's token limit |
| `infer.servers_per_instance` | 16 | in parallel per instance | model: batch slots per GPU; context: https://www.truefoundry.com/blog/vllm-benchmark |
| `infer.service_ms` | 300 | ms | model: abstraction |
| `infer.p99_ratio` | 3 | × median | model: tail multiplier of the service time |
| `infer.queue_per_server` | 64 | waiting per server | model: backlog slots before drops |
| `infer.cost.meter.tokens_in` | 3 | $/million input tokens | source: needed; ballpark; the source says Sonnet 5.5: $2 input per MTok — https://platform.claude.com/docs/en/about-claude/pricing |
| `infer.cost.meter.tokens_out` | 15 | $/million output tokens | source: needed; ballpark; the source says Sonnet 5.5: $10 output per MTok — https://platform.claude.com/docs/en/about-claude/pricing |
| `infer.cost.meter.tokens_in@openai_api` | 2.5 | $/million input tokens | source: needed; ballpark; the source says GPT-6.2 Sol $2.00 input — https://developers.openai.com/api/docs/pricing |
| `infer.cost.meter.tokens_out@openai_api` | 10 | $/million output tokens | https://developers.openai.com/api/docs/pricing — GPT-6.2 Sol $10.00 output |
| `infer.cost.meter.tokens_in@gemini_api` | 1.25 | $/million input tokens | source: needed; ballpark; the source says Gemini 3.1 Pro "$2.00" (≤200k prompt) — https://ai.google.dev/gemini-api/docs/pricing |
| `infer.cost.meter.tokens_out@gemini_api` | 10 | $/million output tokens | source: needed; ballpark; the source says "$12.00" output — https://ai.google.dev/gemini-api/docs/pricing |
| `infer.cost.instance@vllm` | 734 | $/instance-month | https://calculator.holori.com/aws/ec2/g5.xlarge — g5.xlarge "$1.0060/hr ~$734.38/month" |
| `infer.cost.meter.tokens_in@vllm` | 0 | $/million input tokens | billed another way (see the other rows for this flavour) |
| `infer.cost.meter.tokens_out@vllm` | 0 | $/million output tokens | billed another way (see the other rows for this flavour) |
| `infer.cost.instance@triton` | 734 | $/instance-month | https://calculator.holori.com/aws/ec2/g5.xlarge — g5.xlarge |
| `infer.cost.meter.tokens_in@triton` | 0 | $/million input tokens | billed another way (see the other rows for this flavour) |
| `infer.cost.meter.tokens_out@triton` | 0 | $/million output tokens | billed another way (see the other rows for this flavour) |

### `batch`

| Id | Value | Unit | Source |
|---|---|---|---|
| `batch.task_overhead_ms` | 200 | ms per task | model: measured 9–15 ms null tasks; 200 ms is a conservative modelling assumption; scheduling and launching one task; context: https://arxiv.org/pdf/1502.00068 |
| `batch.servers_per_instance` | 4 | in parallel per instance | model: 4 vCPUs on m5.xlarge |
| `batch.service_ms` | 1000 | ms | model: abstraction |
| `batch.p99_ratio` | 3 | × median | model: tail multiplier of the service time |
| `batch.queue_per_server` | 64 | waiting per server | model: backlog slots before drops |
| `batch.cost.instance` | 140 | $/instance-month | source: needed; ballpark; the source says (m5.xlarge $0.192 + EMR $0.048)/h × 730 — https://www.doit.com/compute/spot/us-east-1/m5.xlarge; https://cloudburn.io/tools/amazon-emr-pricing-calculator |

### `media`

| Id | Value | Unit | Source |
|---|---|---|---|
| `media.streams_per_instance` | 500 | streams per instance | https://docs.livekit.io/transport/self-hosting/benchmark/ — 16 vCPU benchmarks scale to ~375–2,800 streams per 2 vCPU; forwarded streams one SFU node carries |
| `media.streams_per_instance@elemental_live` | 8 | streams per instance | model: outputs per channel |
| `media.streams_per_instance@wowza` | 8 | streams per instance | model: license "Up to 10 concurrent, transcoded channels"; context: https://wowza.com/pricing |
| `media.servers_per_instance` | 500 | in parallel per instance | model: abstraction |
| `media.service_ms` | 20 | ms | model: forwarding budget |
| `media.p99_ratio` | 3 | × median | model: tail multiplier of the service time |
| `media.queue_per_server` | 4 | waiting per server | model: backlog slots before drops |
| `media.cost.instance` | 70 | $/instance-month | https://calculator.holori.com/aws/ec2/m5.large/us-east-1 — ≈ $70.08 |
| `media.cost.instance@elemental_live` | 2190 | $/instance-month | https://aws.amazon.com/blogs/media/frequently-asked-questions-about-the-cost-of-live-streaming — $1.99–$4.87 per channel-hour; 2190 = $3.00/h |
| `media.cost.instance@wowza` | 250 | $/instance-month | https://wowza.com/pricing — "$195 per month" + m5.large ≈ $265 |

### `coord`

| Id | Value | Unit | Source |
|---|---|---|---|
| `coord.election_ms` | 1000 | ms | https://etcd.io/docs/v3.5/tuning/ — "By default, etcd uses a 1000ms election timeout"; election timeout (10 × heartbeat) |
| `coord.service_ms` | 2 | ms | https://etcd.io/docs/v3.5/op-guide/performance/ — 1.6 ms average for leader-only writes, 1 client; a write: leader fsync plus a quorum round trip |
| `coord.servers_per_instance` | 64 | in parallel per instance | model: abstraction |
| `coord.p99_ratio` | 4 | × median | model: tail multiplier of the service time |
| `coord.queue_per_server` | 8 | waiting per server | model: backlog slots before drops |
| `coord.cost.instance` | 70 | $/instance-month | https://calculator.holori.com/aws/ec2/m5.large/us-east-1 — ≈ $70.08 |

### `vector`

| Id | Value | Unit | Source |
|---|---|---|---|
| `vector.service_ms` | 2 | ms | model: ANN base time, workload-dependent |
| `vector.servers_per_instance` | 16 | in parallel per instance | model: concurrency knob |
| `vector.p99_ratio` | 4 | × median | model: tail multiplier of the service time |
| `vector.queue_per_server` | 8 | waiting per server | model: backlog slots before drops |
| `vector.failover_s` | 10 | s | model: design assumption |
| `vector.cost.instance` | 260 | $/instance-month | https://www.bytebase.com/dbcost/rds/compare/m-xlarge — db.m5.xlarge PostgreSQL $260 (via bytebase) |
| `vector.cost.instance@pinecone` | 0 | $/instance-month | billed another way (see the other rows for this flavour) |
| `vector.cost.million@pinecone` | 16 | $/million requests | https://www.pinecone.io/pricing/ — Read units "$16–$18 per million" |
| `vector.cost.million_write@pinecone` | 4 | $/million writes | https://www.pinecone.io/pricing/ — Write units "$4–$4.50 per million" |

### `warehouse`

| Id | Value | Unit | Source |
|---|---|---|---|
| `warehouse.concurrency` | 8 | in parallel per instance | https://docs.snowflake.com/en/user-guide/performance-query-warehouse-max-concurrency — Default MAX_CONCURRENCY_LEVEL = 8 |
| `warehouse.scan_gbps` | 1 | GB/s per query | model: no published scan rate (X-Small = 1 node, 8 cores); scan rate of one query on the smallest size; context: https://select.dev/posts/snowflake-warehouse-sizing |
| `warehouse.overhead_ms` | 300 | ms per query | model: compile time is workload-dependent; planning and compilation per query |
| `warehouse.service_ms` | 300 | ms | model: abstraction |
| `warehouse.p99_ratio` | 3 | × median | model: tail multiplier of the service time |
| `warehouse.queue_per_server` | 16 | waiting per server | model: backlog slots before drops |
| `warehouse.cost.instance` | 1460 | $/instance-month | https://docs.snowflake.com/user-guide/warehouses-overview; https://www.owox.com/blog/articles/snowflake-pricing-cost-control — X-Small 1 credit/h × $2 (Standard, AWS us-east-1) × 730 |
| `warehouse.cost.instance@bigquery` | 0 | $/instance-month | billed another way (see the other rows for this flavour) |
| `warehouse.cost.meter.tb_scanned@bigquery` | 6.25 | $/TB scanned | https://cloud.google.com/bigquery/pricing — "$6.25 / 1 tebibyte" |
| `warehouse.cost.instance@redshift` | 182 | $/instance-month | source: needed; ballpark; the source says ra3.large $0.543/h (via vantage) — https://instances.vantage.sh/aws/redshift/ra3.large |
| `warehouse.cost.instance@clickhouse` | 250 | $/instance-month | https://clickhouse.com/pricing — Basic ≈ $160/month; Scale from $499 |
| `warehouse.cost.instance@databricks_sql` | 1000 | $/instance-month | source: needed; ballpark; the source says Serverless SQL 2X-Small 4 DBU/h × $0.70 × 730 — https://learn.microsoft.com/en-us/azure/databricks/resources/pricing |

### `lakehouse`

| Id | Value | Unit | Source |
|---|---|---|---|
| `lakehouse.servers_per_instance` | 1000 | in parallel per instance | model: effectively unlimited; the card's own limits (connections, pools) bind first |
| `lakehouse.service_ms` | 50 | ms | model: abstraction |
| `lakehouse.p99_ratio` | 4 | × median | model: tail multiplier of the service time |
| `lakehouse.queue_per_server` | 4 | waiting per server | model: backlog slots before drops |
| `lakehouse.cost.gb` | 0.023 | $/GB-month | https://aws.amazon.com/s3/pricing/ — S3 Standard $0.023/GB-month (first 50 TB) |

### `tsdb`

| Id | Value | Unit | Source |
|---|---|---|---|
| `tsdb.points_per_s` | 500000 | points/s per instance | source: needed; points written per second on one node; ballpark; the source says Moderate tier "< 250,000" writes/s with "< 1,000,000" series — https://docs.influxdata.com/influxdb/v1/guides/hardware_sizing/ |
| `tsdb.series_limit` | 1000000 | series per instance | https://docs.influxdata.com/influxdb/v1/guides/hardware_sizing/ — Moderate tier series "< 1,000,000"; active series one node holds in memory |
| `tsdb.service_ms` | 1 | ms | model: abstraction |
| `tsdb.servers_per_instance` | 64 | in parallel per instance | model: abstraction |
| `tsdb.p99_ratio` | 4 | × median | model: tail multiplier of the service time |
| `tsdb.queue_per_server` | 8 | waiting per server | model: backlog slots before drops |
| `tsdb.failover_s` | 30 | s | model: open-source InfluxDB has no HA; modelled |
| `tsdb.cost.instance` | 260 | $/instance-month | source: needed; ballpark; the source says r5.xlarge $0.252/h × 730 (via doit) — https://www.doit.com/compute/spot/us-east-1/r5.large |
| `tsdb.cost.instance@timestream` | 0 | $/instance-month | billed another way (see the other rows for this flavour) |
| `tsdb.cost.million@timestream` | 0.01 | $/million requests | source: needed; ballpark; the source says $0.518/TCU-hour; at an assumed ~150 queries/s per TCU, about $1 per million queries — https://aws.amazon.com/timestream/pricing |
| `tsdb.cost.million_write@timestream` | 0.5 | $/million writes | https://aws.amazon.com/timestream/pricing — "$0.50/million writes of 1KiB" |

### `dbproxy`

| Id | Value | Unit | Source |
|---|---|---|---|
| `dbproxy.service_ms` | 0.3 | ms | https://gitlab.com/postgres-ai/postgresql-consulting/tests-and-benchmarks/-/work_items/72 — PgBouncer +0.05 to +0.38 ms |
| `dbproxy.servers_per_instance` | 10000 | in parallel per instance | model: effectively unlimited; the card's own limits (connections, pools) bind first |
| `dbproxy.p99_ratio` | 3 | × median | model: tail multiplier of the service time |
| `dbproxy.queue_per_server` | 8 | waiting per server | model: backlog slots before drops |
| `dbproxy.cost.instance` | 30 | $/instance-month | https://www.doit.com/compute/spot/us-east-1/t3.medium — t3.medium $0.0416/h ≈ $30.37 |
| `dbproxy.cost.instance@rds_proxy` | 45 | $/instance-month | https://www.pump.co/blog/aws-rds-proxy — 4 vCPU × $0.015/h × 730 = $43.80 |

### `graph`

| Id | Value | Unit | Source |
|---|---|---|---|
| `graph.edges_per_ms` | 500 | edges visited per ms | https://neo4j.com/?p=39866 — "millions of traversals per second" on one machine; relationships traversed per ms |
| `graph.service_ms` | 1 | ms | model: abstraction |
| `graph.failover_s` | 30 | s | model: abstraction |
| `graph.servers_per_instance` | 32 | in parallel per instance | model: abstraction |
| `graph.p99_ratio` | 5 | × median | model: tail multiplier of the service time |
| `graph.queue_per_server` | 4 | waiting per server | model: backlog slots before drops |
| `graph.cost.instance` | 260 | $/instance-month | source: needed; ballpark; the source says r5.xlarge ≈ $184 — https://www.doit.com/compute/spot/us-east-1/r5.large |
| `graph.cost.instance@neptune` | 350 | $/instance-month | source: needed; ballpark; the source says db.r6g.large $0.3287/h (via usage.ai) — https://www.usage.ai/blogs/aws/database-savings-plans/neptune-pricing/ |

### `streamproc`

| Id | Value | Unit | Source |
|---|---|---|---|
| `streamproc.concurrency` | 4 | in parallel per instance | model: the default is 1 slot; 4 = slots per core, as configured; context: https://nightlies.apache.org/flink/flink-docs-stable/docs/deployment/config/ |
| `streamproc.records_per_s` | 20000 | records/s per slot | https://dalelane.co.uk/blog/?p=6001 — 1,340–44,320 events/s per CPU; simple records per slot per second |
| `streamproc.service_ms` | 0.05 | ms | model: = 1/20,000 s |
| `streamproc.p99_ratio` | 3 | × median | model: tail multiplier of the service time |
| `streamproc.queue_per_server` | 1000 | waiting per server | model: backlog slots before drops |
| `streamproc.cost.instance` | 70 | $/instance-month | https://calculator.holori.com/aws/ec2/m5.large/us-east-1 — ≈ $70.08; managed Flink $0.11/KPU-h; context: https://aws.amazon.com/kinesis/analytics/pricing |

### `mqtt`

| Id | Value | Unit | Source |
|---|---|---|---|
| `mqtt.max_connections` | 1000000 | connections per instance | https://docs.emqx.com/en/enterprise/v4.0/tutorial/tune.html — "1.3 million on an 8-core, 32G" server |
| `mqtt.conn_memory_kb` | 8 | KB per connection | https://www.emqx.com/en/blog/emqx-single-node-supports-5m-connections — 48.7 GB for 5M connections ≈ 10 KB |
| `mqtt.msgs_per_s` | 100000 | messages/s per instance | source: needed; ballpark; the source says 4 vCPU / 8 GiB QoS 0: "60K" TPS — https://docs.emqx.com/en/emqx/latest/performance/performance-reference.html |
| `mqtt.service_ms` | 2 | ms | https://docs.emqx.com/en/emqx/latest/performance/performance-reference.html — QoS 0 average "2.079" ms |
| `mqtt.servers_per_instance` | 10000 | in parallel per instance | model: effectively unlimited; the card's own limits (connections, pools) bind first |
| `mqtt.p99_ratio` | 3 | × median | model: tail multiplier of the service time |
| `mqtt.queue_per_server` | 4 | waiting per server | model: backlog slots before drops |
| `mqtt.cost.instance` | 70 | $/instance-month | https://calculator.holori.com/aws/ec2/m5.large/us-east-1 — ≈ $70.08 |
| `mqtt.cost.instance@aws_iot_core` | 0 | $/instance-month | billed another way (see the other rows for this flavour) |
| `mqtt.cost.meter.conn_minutes@aws_iot_core` | 0.08 | $/million connection-minutes | https://aws.amazon.com/iot-core/pricing/ — "$0.08/1,000,000 minutes of connection" |
| `mqtt.cost.meter.messages@aws_iot_core` | 1 | $/million messages | https://aws.amazon.com/blogs/aws/aws-iot-update-better-value-with-new-pricing-model — "starts at $1 per million messages" |

### `connector`

| Id | Value | Unit | Source |
|---|---|---|---|
| `connector.concurrency` | 4 | in parallel per instance | model: tasks.max default 1; 4 as configured; context: https://docs.confluent.io/platform/current/installation/configuration/connect/source-connect-configs.html |
| `connector.service_ms` | 50 | ms | model: abstraction |
| `connector.p99_ratio` | 4 | × median | model: tail multiplier of the service time |
| `connector.queue_per_server` | 8 | waiting per server | model: backlog slots before drops |
| `connector.cost.instance` | 70 | $/instance-month | https://calculator.holori.com/aws/ec2/m5.large/us-east-1 — ≈ $70.08 |
| `connector.cost.instance@fivetran` | 0 | $/instance-month | billed another way (see the other rows for this flavour) |
| `connector.cost.million@fivetran` | 500 | $/million requests | https://www.eon.io/blog/fivetran-pricing — "$495 per million MAR" (Standard, first million) (via eon) |

### `config`

| Id | Value | Unit | Source |
|---|---|---|---|
| `config.cost.instance` | 20 | $/instance-month | model: no per-instance price; AWS AppConfig bills per request ($0.20 per million requests, $800 per million configurations received); context: https://aws.amazon.com/systems-manager/pricing |
