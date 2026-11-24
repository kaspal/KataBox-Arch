## 1

Where does an EU user's history go on a normal day, and where does it go after the database fails over? And which GPU answers an EU message, on a normal day and when a Region is lost? A replica is where the primary's job lands when it is lost, and a standby Region is where the traffic lands: both must obey the same rule as the primary. When a whole Region is lost, its GPUs go with it, and what is left must carry every message.

## 2

The DNS card sends EU users to every Region behind it, and an App Server takes turns between every Model Inference card it is wired to. So every GPU an App Server can reach, and every Region the DNS card can send users to, must be in a Region tagged `eu`. A GPU pool in us-east would answer EU messages, on a normal day or after a failover. US users may be answered in the EU. Use two EU Regions (say eu-west and eu-central), each with App Servers and a GPU pool, and keep the history primary in one and its replica in the other. For the GPUs, count tokens: 30 messages a second of about 250 tokens is 7,500 tokens a second or more, and a GPU with a full batch writes at most 64 × 40 = 2,560. When one Region is lost, the other's pool must carry all of it, so each pool needs about 4 GPUs, and each App Server should be wired to both pools.

## 3

A Web client (Timeout 30,000 ms, 2 Retries, Retry backoff 200 ms, Backoff jitter Full) calls a Route 53 DNS card (TTL 30 s, Routing Round robin) that points at two Regions, both with the Residency tag `eu`: eu-west and eu-central. Each holds an AWS ALB, 4 Go App Servers and a vLLM on GPU card with 4 Instances and Batch slots per GPU at 64. Wire each App Server tier to both vLLM cards. Put a PostgreSQL primary in eu-west and a PostgreSQL replica in eu-central, wired with a Replicate edge, both with Failover time 10 s. Wire both App Server tiers to the primary with Query edges.
