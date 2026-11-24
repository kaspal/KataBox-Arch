# Editorial

## Where the load goes

Three things decide this design: where the history is written, which GPUs answer EU messages, and how many GPUs are left when something is lost.

The history first. Every message makes two writes, the user's message and the answer. At the peak that is 30 × 2 = 60 writes a second, and 24 of them belong to EU users. The DNS card spreads users over the Regions with no regard for where they live, so an EU message is served in every Region the DNS card points at. The only way to keep EU history in the EU is for every App Server, wherever it runs, to write to a database in an `eu` Region. US users' history has no rule, so one EU database can hold everyone's.

A database has a primary and replicas, and the rule follows the primary's job. When the primary is lost, a replica is promoted, and from then on every write lands on it. So the replica's Region matters as much as the primary's: it must be tagged `eu` too, and it must be a different Region from the primary's, or a lost Region takes both.

Then the model calls. Processing counts for residency just as storage does: the GPU that reads an EU message and writes its answer must sit in an `eu` Region. Nothing routes a message by its user's residency. The DNS card sends EU users to every healthy Region behind it, and an App Server takes turns between all the Model Inference cards it is wired to. So the rule reaches every GPU any App Server can call, and every Region the DNS card can send users to, today or after a failover. A GPU pool in us-east would answer about half of the EU messages on a normal day if the App Servers were wired to it, and all of them if it were a standby that takes over when the EU goes dark. The only safe place for the GPUs is EU Regions, and US users are answered there too.

Then the GPU count. Before the first token, a message needs 200 ms plus its prompt at 5,000 tokens a second: 0.2 + 800 / 5,000 = 0.36 s for a typical prompt, 0.6 s for a long one, plus about 70 ms each way when the App Server calls the GPU in the other Region. The limit is 1 s. The answer is then written at 80 tokens a second alone and 40 with a full batch of 64, so one GPU writes at most 64 × 40 = 2,560 tokens a second. The peak asks for 30 × 250 = 7,500 tokens a second, and more in practice, since long answers pull the average above the typical 250. Three GPUs at 100% are not quite enough; four are about three-quarters full.

On a normal day two pools share that work. On a bad day one pool does it alone, when either EU Region goes dark. So each pool must carry the whole peak by itself.

## The reference design

A Web client with a Timeout of 30,000 ms, 2 Retries, a Retry backoff of 200 ms and the Backoff jitter set to Full (random wait) calls a Route 53 DNS card (TTL 30 s, Round robin). It points at two Regions, both with the Residency tag `eu`:

- **eu-west**: an AWS ALB, 4 Go App Servers, a vLLM on GPU card with 4 Instances and Batch slots per GPU at 64, and the history's PostgreSQL primary.
- **eu-central**: the same, with a PostgreSQL replica in place of the primary, wired from the primary with a Replicate edge.

Both databases have a Failover time of 10 s. Both App Server tiers write to the primary with Query edges, and each tier is wired to both vLLM cards. There is no US Region: a GPU there could answer an EU message and break the rule, so the US users are answered from Europe too.

It costs about $7,002 per month: $5,872 for the 8 GPUs, $520 for the two databases, $560 for the 8 App Servers and $50 for the two ALBs.

On a normal day each GPU pool is about 24% busy, the time to first token (TTFT) p95 is about 0.79 s, the whole answer's p95 is about 9.7 s, there are no errors and no residency violations.

"The Region with the history database goes dark" loses eu-west 30 seconds into a run of 5 minutes, for good: its ALB, App Servers, GPUs and the primary. For a few seconds the DNS card still sends half the users there, until two health checks in a row find it down, and every write waits for the replica to take over 10 s later. Those seconds give about 4.6% errors, most of them retried by the client. Then eu-central's replica is the primary, and eu-central's GPUs carry all 30 messages a second, both still in the EU: the TTFT p95 is about 0.65 s (no call crosses a Region any more), the whole answer's p95 about 11.8 s, and there are no residency violations.

"An EU-only audit day: the database primary fails, then the Region it moved to" sends every message from an EU user at the same total rate, so every one of the 60 writes and 30 model calls a second must stay in the EU. The run lasts 7.5 minutes. The primary is stopped 30 seconds in, for 40 seconds, and the replica in eu-central takes over; the old primary comes back as a replica. Then, from 100 seconds in, the Region that now holds the primary, eu-central, goes dark for 60 seconds: its ALB, App Servers, GPUs and the new primary. The database fails back to eu-west, and eu-west's GPUs carry every message alone. About 6.2% of messages fail around the two failures, the TTFT p95 is about 0.74 s, the whole answer's p95 about 9.5 s, and there are no residency violations.

There is no cost challenge for this problem: a search for a cheaper design that passes every test found none below 80% of the reference's cost.

## Why the first try breaks

A common first try gets the database right and keeps the GPUs out of the US on a normal day. It runs everything in eu-west: an ALB, 4 Go App Servers, a vLLM card with 4 GPUs and the PostgreSQL primary, with the replica in eu-central. For disaster recovery it keeps a standby in us-east, next to the company's US servers: an ALB, 4 App Servers and a vLLM card with 4 GPUs, wired only to each other and to the EU primary. The DNS card's Routing is set to Failover (first healthy), with eu-west first, so us-east gets no traffic while eu-west is up. It costs the same $7,002 per month and passes "A normal day": every message is served in eu-west, the GPUs there are about 58% busy, the TTFT p95 is about 0.65 s and the whole answer's p95 about 11.6 s.

When eu-west goes dark, the DNS card sends everyone to us-east. The history is fine: the replica in eu-central is promoted, and every write still lands in the EU. But every EU message is now answered by the GPUs in us-east. The errors are close to the reference's (about 5.8%), the latency is fine, and the test fails anyway: about 3,100 residency violations, one for every EU message from the moment the DNS card switches to the end of the run. The audit test passes, because eu-west never goes dark in it. Disaster recovery moved the processing to the place it must never go.

Other first tries break in instructive ways:

- **A GPU pool on each continent.** Regions us-east and eu-west, each with App Servers and 4 GPUs, every App Server wired to both pools, and the database in eu-west and eu-central: the design that fits a rule about stored history only. It fails "A normal day" at once: the App Servers take turns between the pools, so about half of the EU messages are answered in us-east, about 340 violations in the one-minute test. When eu-west goes dark, all of them are: about 3,300.
- **The replica in us-east.** The reference with its replica moved to a us-east Region fails both hidden tests: once the replica is promoted, both writes of every EU message land in a Region tagged `us`. That is about 6,200 violations when eu-west goes dark, and about 3,300 in the audit test.
- **Everything in us-east.** One US Region with the GPUs and the database fails "A normal day" at once: every EU write and every EU model call is a violation.
- **No replica.** A single PostgreSQL in eu-west ($6,742) passes the normal day and loses every write when it goes: about 91% errors when eu-west goes dark, about 24% in the audit test.
- **Pools sized for half the peak.** Two vLLM cards of 3 GPUs each ($5,534) pass the normal day. When eu-west goes dark, 3 GPUs get the whole peak, more tokens a second than they can write: the batches fill, messages queue, and the test ends with about 41% errors and a p95 of about 28 s. The audit test fails on the time to first token (a TTFT p95 of about 3 s) while eu-central is dark.
- **One big GPU card.** All 8 GPUs in one vLLM card in eu-west cost the same and pass the audit test, since eu-west keeps running there. When eu-west goes dark, nothing is left to answer: about 91% errors.

The point is this: a residency rule is a rule about every place the data can end up, and that includes the GPUs that process it, not only the databases that keep it. Failover is one of those places: put the replica, the standby Region and anything else that can take over a job in a Region with the same tag. And a Region you can lose is capacity you can lose: each GPU pool must carry the whole peak by itself.

## What the simulator leaves out

The quality of the answers is not simulated: a GPU in either Region gives the same answer. Users have no location: a DNS card spreads them over the Regions in turn, with no routing by geography, and an App Server can't pick a GPU by the user's residency. So the simulator can't send EU users to EU GPUs and US users to US GPUs on purpose, and US users get no slower answers from Europe here. A real service would add geographic routing and a US GPU pool that only US users reach. Residency is checked when a database, cache or queue stores or serves a request and when a Model Inference card answers one; copying data by replication is not counted, so a replica outside the EU breaks no rule in the simulator until it is promoted, while in reality it holds copies of EU history from the first row it copies. Logs, backups and metrics are not checked for residency at all. Real residency rules come from contracts and laws such as the GDPR; this problem's rule is the one in its statement, not legal advice.

GPU memory is not simulated: the limit of 64 answers at once on one GPU comes from the statement. The 10 s failover is set in the statement; a real database's failover time depends on its setup. A Region is lost all at once; real outages are often partial, and they can last hours where the tests run minutes. Starting GPU servers and loading the model are not simulated, and a Model Inference card can't be autoscaled here. The GPU price is the registry's teaching figure (a GPU node at $734 a month), not a quote.

## Things to try

- Move the first try's standby from us-east to eu-central (Residency tag `eu`) and run "The Region with the history database goes dark" again.
- Add a us-east Region with a vLLM card, wire the reference's App Servers to it, and run "A normal day". Count the violations.
- Give the reference's vLLM cards 3 Instances each and run "The Region with the history database goes dark". Watch the batches fill.
