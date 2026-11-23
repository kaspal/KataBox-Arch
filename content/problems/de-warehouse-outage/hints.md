## 1

If the App Server loads each batch into Snowflake itself, what happens to the batches that arrive during the 60 s it is down? Something has to hold them until it is back. And once it is back, how many batches are waiting, and how fast can they go in while new ones keep arriving?

## 2

Publish each batch to a Stream and load it from there with a Data Connector: a failed load stays in the Stream and is retried, so nothing is lost. Then size for the catch-up, not for the normal day. At the peak, 45 × 60 = 2,700 batches pile up. The backlog only shrinks by what the pipeline loads beyond the 45 a second that keep arriving: drain time = backlog ÷ (catch-up rate − live rate). For 50 s you need about 45 + 2,700 ÷ 50 ≈ 99 loads a second. Snowflake loads about 8 ÷ 0.3 ≈ 27 a second per instance.

## 3

Wire the App Servers with a Publish edge to an Apache Kafka Stream with 48 partitions, consumed by Kafka Connect with 3 instances and a concurrency of 16 (48 slots), loading into Snowflake with 4 instances (32 loads at once, about 107 a second). Keep the partitions and the connector's slots above the warehouse's 32, so the warehouse is the only cap. Put an AWS ALB and 2 Go App Servers in front.
