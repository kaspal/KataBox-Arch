# Editorial

## Where the load goes

400 clicks a second is not much traffic for the collector or for Kafka. The work is in the Stream Processor, and it has two costs.

The first is processing. A sliding window of 60 s that slides every 1 s updates 60 results for every click, so each click costs 60 simple records: 400 × 60 = 24,000 records a second, a little more than one slot at 20,000 a second. At four times the users that is 96,000 records a second, about 5 slots.

The second is state. Every user with a click in the last 60 s has an open session: 400 × 60 = 24,000 sessions. At 128 KB each, that is 24,000 × 128 KB ≈ 2.9 GB. If Memory × Instances is smaller, the state spills to disk and every record takes 4 times longer. Spilling turns those 5 slots into 20.

Then there is the crash. A stream processor saves its progress (its state and its place in the stream) at every checkpoint. When it dies, it restarts from the last checkpoint and processes again everything that came after it, so those results reach the sessions table a second time. The Checkpoint interval sets how much that can be: at 400 clicks a second, a 10 s interval replays at most about 4,000 results, a 600 s interval up to 240,000.

## The reference design

2 Go App Servers write the clicks to an Apache Kafka Stream with 12 partitions. An Apache Flink Stream Processor with 3 instances (4 slots each, so 12 slots, one per partition) reads it with Window `sliding`, Window length 60 s, Slide 1 s, Watermark delay 5 s, State per key 128 KB, Memory 8 GB and Checkpoint interval 10 s. It writes each session to a ScyllaDB NoSQL Store with "Idempotent writes (dedup)" on: a session is upserted by user, so writing the same result again changes nothing. This costs about $1,503 per month: $140 for the App Servers, $153 for Kafka, $210 for Flink and $1,000 for ScyllaDB.

The state of 2.9 GB fits easily in 3 × 8 GB = 24 GB. That is far more than today's state needs, and it is on purpose: with four times the users, the real state would be about 11.7 GB, and it still fits. A normal day has a p95 of about 5.9 s from click to table (up to a second waiting for the next slide, plus the 5 s watermark), no errors, and the processor is about 15% busy.

"The stream processor crashes" kills the Stream Processor 50 seconds in, for 10 seconds, in a run of 120 seconds. The clicks wait in Kafka and nothing is lost. On restart the processor replays from its last checkpoint, but the sink deduplicates, so the run counts no duplicates. Even without dedup, the 10 s checkpoint keeps the replay under the limit: a variant of the reference without dedup replays about 4,000 results.

"Four times as many users" runs the whole test at 1,600 clicks a second. The processor works about four times as hard, around 60% busy, and the p95 stays at about 5.9 s with no errors.

For the cost challenge, the search found no design under 80% of the reference's cost (its best came to 86%), so this problem has no cost challenge. The NoSQL Store is most of the bill, and the processor is already cheap.

## Why the first try breaks

A common first try has the same shape but sizes the processor like a small job: an Apache Flink Stream Processor with 2 instances of 1 GB Memory, the Checkpoint interval at 600 s ("checkpoints are expensive, do them rarely"), and a ScyllaDB NoSQL Store without dedup. It costs about $1,433 and passes "A normal day" with a p95 of about 5.9 s.

It already spills on a normal day: 2.9 GB of state does not fit in 2 × 1 GB, so every record is 4 times slower. The processor is about 74% busy where the reference is 15%, and nothing shows it yet.

With four times the users, it needs about four times that, roughly three times what its 8 slots can do. Kafka keeps the clicks, so none are lost, but the backlog grows for the whole test: the p95 climbs to about 59 s against a limit of 10 s (verdict "SLO breach"). Giving it the memory fixes it: the same design with 8 GB of Memory passes, and so does one with 3 instances of 1 GB, because 3 GB is just enough for 2.9 GB.

The crash breaks it too. The 600 s checkpoint has not come yet when the processor dies at 50 s, so it replays everything it has done since the start: about 20,300 results written to the sessions table twice, over three times the limit of 6,000 (verdict "Dropped requests"). The same design with a 10 s checkpoint replays about 4,000 and passes.

The point is this: a stateful stream processor is sized by its state as much as by its records. Work out keys × bytes per key, give it memory with room for growth, and remember that a spilling job can look healthy until the load rises. And the checkpoint interval is how much work you are willing to redo after a crash; an idempotent sink makes the redone work harmless.

## What the simulator leaves out

The simulator has no session windows. Sessions here are a sliding window of 60 s standing in for a 30-minute inactivity gap, so time is scaled down, and the number of open sessions is the clicks in one window (at most the number of users). The state size is worked out from the stated peak of 400 clicks a second, not from a test's traffic, so in "Four times as many users" the state does not grow; what grows is the work, and a spilling processor cannot keep up. Spilling is a flat 4 times slower, a modelling choice; real RocksDB state backends degrade more gradually. Checkpoints cost nothing here: real checkpoints take time, I/O and storage, which is why teams do not checkpoint every second. Exactly-once transactional sinks are not simulated; the simulator counts the duplicates instead. A Stream Processor instance costs the same whatever its Memory, so more memory is free here; it is not in a real cluster. Recovery time after a restart (restoring state from the checkpoint) is not simulated.

## Things to try

- Give the reference 2 instances of 1 GB Memory and run "A normal day". It passes, but compare the processor's load (about 74%) with the reference's 15%.
- Turn dedup off in the reference and run the crash with a Checkpoint interval of 10 s, 30 s and 60 s. Watch the duplicates grow with the interval.
- Change the Slide to 5 s. The processor does a fifth of the work, but each session is updated only every 5 s and 2,000 results reach the NoSQL Store at once: the p95 rises to about 9.9 s, right at the limit.
