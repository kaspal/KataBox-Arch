## 1

Work out how big the processor's state is: how many sessions are open in a 60 s window, times 128 KB each. Then compare it with the memory you gave the processor. And ask what happens to the sessions table when the processor restarts and has to redo the work since it last saved its progress.

## 2

400 clicks a second over a 60 s window is 24,000 open sessions; at 128 KB each that is about 2.9 GB. If that is more than Memory × Instances, every record is 4 times slower: fine on a quiet day, too slow when the users grow. On a crash, the processor goes back to its last checkpoint and writes everything since then again, so the Checkpoint interval decides how much is repeated. Writes that upsert by key make the repeats harmless.

## 3

Use an Apache Kafka Stream with 12 partitions and an Apache Flink Stream Processor with 3 instances, Memory 8 GB, Checkpoint interval 10 s, Window `sliding`, Window length 60 s, Slide 1 s, Watermark delay 5 s and State per key 128 KB. Write to a ScyllaDB NoSQL Store with "Idempotent writes (dedup)" on. Keep 2 Go App Servers in front.
