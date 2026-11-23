## 1

Rewinding the live job puts a month of old events in front of every new one. How long does a live event wait behind them, and what happens when the replay is sped up, or when one of the processors that does both jobs goes away?

## 2

Give the replay its own consumer group: a second Stream Processor card that reads the retained events from its own Stream (its own offsets) and writes with the fixed code, while the live job keeps reading only new events. Then nothing the replay does, going faster or crashing, reaches the live events. Size the live side for the evening peak, and keep a spare: when one live Stream Processor card is lost, the other must carry the live events alone.

## 3

Put an AWS ALB in front of 2 Go App Servers that publish to an Apache Kafka Stream with 16 Partitions (Retention 720 h). Two Apache Flink Stream Processors, each with 2 instances (Concurrency per instance 4, Records per second per slot 200), consume it and write to PostgreSQL. Wire the Airflow Scheduler to a second Apache Kafka Stream with 8 Partitions, consumed by a third Flink card with 2 instances and the same settings, which writes to the same PostgreSQL.
