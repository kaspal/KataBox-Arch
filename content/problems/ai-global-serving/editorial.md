# Editorial

## Where the load goes

Two things decide this design: where the history is written, and how many GPUs are left when something is lost.

The history first. Every message makes two writes, the user's message and the answer. At the peak that is 30 × 2 = 60 writes a second, and 24 of them belong to EU users. The DNS card spreads users over the Regions with no regard for where they live, so an EU message is served in us-east about half the time. The only way to keep EU history in the EU is for every App Server, wherever it runs, to write to a database in an `eu` Region. US users' history has no rule, so one EU database can hold everyone's. A write from us-east to the EU costs about 70 ms each way, small next to an answer that takes seconds.

A database has a primary and replicas, and the rule follows the primary's job. When the primary is lost, a replica is promoted, and from then on every write lands on it. So the replica's Region matters as much as the primary's: it must be tagged `eu` too, and it must be a different Region from the primary's, or a lost Region takes both.

Then the GPUs. Before the first token, a message needs 200 ms plus its prompt at 5,000 tokens a second: 0.2 + 800 / 5,000 = 0.36 s for a typical prompt, 0.6 s for a long one, plus about 70 ms each way when the App Server calls a GPU in the other Region. The limit is 1 s. The answer is then written at 80 tokens a second alone and 40 with a full batch of 64, so one GPU writes at most 64 × 40 = 2,560 tokens a second. The peak asks for 30 × 250 = 7,500 tokens a second, and more in practice, since long answers pull the average above the typical 250. Three GPUs at 100% are not quite enough; four are about three-quarters full.

On a normal day two pools share that work. On a bad day one pool does it alone: when eu-west goes dark, or when a vLLM card is stopped. So each pool must carry the whole peak by itself.

## The reference design

A Web client with a Timeout of 30,000 ms, 2 Retries, a Retry backoff of 200 ms and the Backoff jitter set to Full (random wait) calls a Route 53 DNS card (TTL 30 s, Round robin). It points at two Regions:

- **us-east** (Residency tag `us`): an AWS ALB, 4 Go App Servers and a vLLM on GPU card with 4 Instances and Batch slots per GPU at 64.
- **eu-west** (Residency tag `eu`): the same, plus the history's PostgreSQL primary.

A third Region, **eu-central** (Residency tag `eu`), holds a PostgreSQL replica, wired from the primary with a Replicate edge. Both databases have a Failover time of 10 s. Both App Server tiers write to the primary with Query edges, and each tier is wired to both vLLM cards.

It costs about $7,182 per month: $5,872 for the 8 GPUs, $700 for the two databases, $560 for the 8 App Servers and $50 for the two ALBs.

On a normal day each GPU pool is about 24% busy, the time to first token (TTFT) p95 is about 0.77 s, the whole answer's p95 is about 9.7 s, there are no errors and no residency violations.

"The Region with the history database goes dark" loses eu-west 30 seconds into a run of 5 minutes, for good: its ALB, App Servers, GPUs and the primary. For a few seconds the DNS card still sends half the users there, until two health checks in a row find it down, and every write waits for the replica to take over 10 s later. Those seconds give about 4.6% errors, most of them retried by the client. Then eu-central's replica is the primary, still in the EU, and the us-east GPUs carry all 30 messages a second: the TTFT p95 is about 0.76 s, the whole answer's p95 about 12.1 s, and there are no residency violations.

"An EU-only audit day: the database primary fails, then a GPU pool" sends every message from an EU user at the same total rate, so every one of the 60 writes a second must land in the EU. The primary is stopped 30 seconds in, for good, and the replica in eu-central takes over; then, from 100 seconds in, the first vLLM card (by card id) is stopped for 60 seconds, and the App Servers send every model call to the other card. About 4.7% of messages fail around the two failures, the TTFT p95 is about 0.82 s, the whole answer's p95 about 10.5 s, and there are no residency violations.

There is no cost challenge for this problem: a search for a cheaper design that passes every test found none below 80% of the reference's cost. The cheaper designs that do pass keep a smaller GPU pool exactly where the GPU test happens to strike, which works by luck of the card names, not by design.

## Why the first try breaks

A common first try gets almost everything right. It has the same two Regions, the same GPUs and the same primary in eu-west, and it puts the database's replica in us-east, next to the company's US servers, as a disaster-recovery copy. It costs the same $7,182 per month, and it passes "A normal day" with the reference's numbers: every write goes to the primary in eu-west, so no write lands outside the EU.

When eu-west goes dark, the replica in us-east is promoted, and from then on both writes of every EU message land in a Region tagged `us`. The errors are the reference's (about 4.6%), the latency is fine, and the test fails anyway: about 6,200 residency violations, every EU message for the last four minutes of the run. In the audit test, when the primary is stopped, the same promotion happens with only EU traffic: about 15,600 violations. Disaster recovery moved the data to the place it must never go.

Other first tries break in instructive ways:

- **Everything in us-east.** One US Region with the GPUs and the database fails "A normal day" at once: every EU write is a violation.
- **No replica.** A single PostgreSQL in eu-west ($6,832) passes the normal day and loses every write when it goes: about 91% errors in both hidden tests.
- **Pools sized for their own users.** Two vLLM cards of 3 GPUs each ($5,714) pass the normal day. When eu-west goes dark, 3 GPUs get the whole peak, more tokens a second than they can write: the batches fill, messages queue, and the test ends with about 41.6% errors and a p95 of about 28 s. The audit test fails on latency (p95 about 17.6 s) while one card is stopped.
- **One big GPU card.** All 8 GPUs in one vLLM card in us-east cost the same and pass the region test, since us-east keeps running. When the card is stopped, nothing is left: about 26% errors in the audit test.
- **Each App Server tier wired only to its own Region's GPUs.** It passes, barely: while the eu-west card is stopped, the DNS card still sends half the users to App Servers with no GPU to call, and only client retries that land in us-east get an answer (about 7.2% errors, against a limit of 8%).

The point is this: a residency rule is a rule about every place the data can end up, and failover is one of those places. Put the replica, and anything else that can take over the primary's job, in a Region with the same tag. And a Region you can lose is capacity you can lose: each GPU pool must carry the whole peak by itself.

## What the simulator leaves out

The quality of the answers is not simulated: a GPU in either Region gives the same answer. Users have no location: a DNS card spreads them over the Regions in turn, with no routing by geography, so EU users don't get faster answers from eu-west, and the simulator can't send their messages there on purpose. Residency is checked only when a database, cache or queue stores or serves a request; copying data by replication is not counted, so the first try's replica breaks no rule in the simulator until it is promoted, while in reality it holds copies of EU history from the first row it copies. Model calls, logs, backups and metrics are not checked for residency at all. Real residency rules come from contracts and laws such as the GDPR, which often cover processing too; this problem's rule is the one in its statement, not legal advice.

GPU memory is not simulated: the limit of 64 answers at once on one GPU comes from the statement. The 10 s failover is set in the statement; a real database's failover time depends on its setup. A Region is lost all at once and stays lost; real outages are often partial, and they can last hours where the test runs 5 minutes. Starting GPU servers and loading the model are not simulated, and a Model Inference card can't be autoscaled here. The GPU price is the registry's teaching figure (a GPU node at $734 a month), not a quote.

## Things to try

- Move the first try's replica from us-east to a new Region tagged `eu` and run both hidden tests again.
- Give the reference's vLLM cards 3 Instances each and run "The Region with the history database goes dark". Watch the batches fill.
- Remove the edges from each App Server tier to the other Region's vLLM card and compare the audit test's errors with the reference's.
