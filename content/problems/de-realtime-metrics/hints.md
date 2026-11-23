## 1

A window can't wait forever for its stragglers, but a phone can be two minutes late. What happens to an event that arrives after its window has closed? And when three times the events arrive, how many of them can the processor take at once?

## 2

Size the Allowed lateness from the delay distribution, not from the watermark: mobile events reach 120 s late at p99, so let a closed window take updates for longer than that. Keep the Watermark delay short, because every second of it is added to every result. For throughput, count slots: about 800 events a second each, and the Stream's Partitions cap how many can work.

## 3

Put an AWS ALB in front of 4 Go App Servers that publish to an Apache Kafka Stream with 32 Partitions and read the dashboard from a TimescaleDB Time-Series Database. Consume the Stream with an Apache Flink Stream Processor of 8 instances with a Concurrency per instance of 4: Window Tumbling, Window length 30 s, Watermark delay 5 s, Allowed lateness 180 s. Wire the processor to the Time-Series Database with a Query edge.
