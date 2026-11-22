A stream is an append-only log. Producers add events to the end, and the log keeps them for a set time whether or not anyone has read them. Each consumer remembers its own position, so many systems can read the same events independently, and a new one can start from the beginning.

A stream is split into **partitions**. Events with the same key always land in the same partition, which keeps them in order, and each partition is read by one consumer of a group at a time. The partition count is therefore the ceiling on how much work can happen in parallel.

Streams shine for high volumes of events that several parts of a system care about: activity feeds, location updates, changes captured from a database. For a plain list of independent jobs, a queue is usually simpler.

**How the simulator bills this.** Kafka is Amazon MSK at $153 per broker-month (kafka.m5.large), and a stream card is one broker. There is no fee per message, and partitions are free. Kinesis bills $10.95 per shard-month, one shard per partition, plus $0.014 per million PUT units of 25 KB, so on Kinesis extra partitions cost money. Storage is not modelled (list price, us-east-1, read 2026-11-22).
