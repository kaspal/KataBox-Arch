## 1

Where does an EU user's history go on a normal day, and where does it go after the database fails over? A replica is where the primary's job lands when it is lost, so the replica must obey the same rule as the primary. And when a whole Region is lost, its GPUs go with it: what is left must carry every message.

## 2

Keep the history primary in a Region tagged `eu`, and put its replica in a second Region that is also tagged `eu` (say eu-west and eu-central), not next to your US servers. Every App Server, in the US or the EU, writes to that one EU database: US users' history may live in the EU too. For the GPUs, count tokens: 30 messages a second of about 250 tokens is 7,500 tokens a second or more, and a GPU with a full batch writes at most 64 × 40 = 2,560. When one GPU pool is lost, the other must carry all of it, so each pool needs about 4 GPUs, and each App Server should be wired to both pools.

## 3

A Web client (Timeout 30,000 ms, 2 Retries, Retry backoff 200 ms, Backoff jitter Full) calls a Route 53 DNS card (TTL 30 s) that points at two Regions: us-east (Residency tag `us`) and eu-west (tag `eu`). Each holds an AWS ALB, 4 Go App Servers and a vLLM on GPU card with 4 Instances and Batch slots per GPU at 64. Wire each App Server tier to both vLLM cards. Put a PostgreSQL primary in eu-west and a PostgreSQL replica in a third Region, eu-central (tag `eu`), wired with a Replicate edge, both with Failover time 10 s. Wire both App Server tiers to the primary with Query edges.
