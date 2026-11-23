# Editorial

## Where the load goes

30,000 sensors send a point every 10 seconds: 3,000 points a second. Each point makes two calls: one to the classifier for its score, then one write to the Time-Series Database. Scoring a point holds a Triton batch slot for about 12 ms (the time to first token for about 30 tokens in; the one score out comes with it), so 3,000 × 0.012 s keeps about 36 scores in flight. Whoever calls the classifier waits those 12 ms too, holding one of its own slots. The database is easy on a normal day: one node writes 20,000 points a second.

The hidden tests push on two different places. In the storm the points arrive 5 times as fast for 20 seconds (from 20 s in): 15,000 a second and about 180 scores in flight, which needs about 3 GPUs at 64 batch slots each before any headroom. With the firmware update the 300,000 series become 2,400,000 for 20 seconds. One node indexes 1,000,000; past 2,000,000 it refuses the writes, and a point that isn't stored raises no alert.

So the questions are: who waits for the model, how many GPUs does a storm need, and how many database nodes do the series need?

## The reference design

The sensors keep their MQTT connections on an EMQX IoT Broker with the default 2 instances. Points enter at a Go App Server (2 instances), which only publishes each one to an Apache Kafka Stream with 384 partitions. A Celery Worker with 6 instances, Concurrency 64 per instance (384 threads, one per partition) and a Service time of 1 ms consumes the stream, calls an NVIDIA Triton Model Inference card (Model type Classifier) with 4 instances, and writes the point and its score to an InfluxDB Time-Series Database with 3 instances at 20,000 points per second each. It costs about $4,569 per month, and the GPUs are most of it: Triton is $2,936, InfluxDB $780, the workers $420, Kafka $153, and the broker and the App Servers $140 each. A normal day has an alert latency p95 of about 38 ms and no errors; the GPUs are about 14% busy, the workers about 16%, the App Servers about 14%.

Why a Worker and not a Stream Processor? The consumer does almost nothing itself: it holds a slot while it waits for the model and the database, about 20 ms a point, so a storm keeps about 300 slots busy. A worker thread that waits is cheap, and 64 of them fit on one instance. A Stream Processor's slots are cores: Apache Flink is priced at $70 for every 4 slots, and the storm needs close to 300 of them: 36 instances with Concurrency 8 (288 slots) pass, but they cost $5,040 a month, and the design goes over the budget.

"A storm: the sensors send 5 times as many points" is where the sizing shows. The App Servers only publish, so five times the points take them to about 70% busy, not past their limit. The workers' threads and the GPUs carry the scoring: about 180 scores in flight on 256 batch slots is about 70% busy, so no point waits for a slot. The workers are about 80% busy. The p95 stays at about 38 ms, with no errors. Three GPUs would also pass, at about 94% busy in the storm; the fourth is headroom, for a storm a little stronger than this one.

"A firmware update multiplies the series by 8" is the series budget. With 3 instances, each node holds 2,400,000 / 3 = 800,000 series during the explosion, under its limit of 1,000,000. Writes keep their normal speed, and the p95 stays at about 39 ms with no errors. The points per second never needed 3 nodes; the series did. Two nodes would hold 1,200,000 each: over the limit, but under twice it, so they write four times slower instead of refusing. This load survives that (the p95 rises to about 50 ms), but a budget kept under the limit leaves room for a worse label.

The cost challenge search found no design under 80% of the reference's cost (the best it reached was $3,695, 81%), so this problem has no cost challenge. The GPUs set the floor: three of them are needed for the storm whatever else changes.

## Why the first try breaks

A common first try has the same IoT Broker and App Servers, with the App Server scoring every point inline: it calls Triton with the default 2 instances, then writes to one InfluxDB node. No stream. It costs about $2,008 and passes "A normal day" with a p95 of about 35 ms. But the App Servers are already about 56% busy: each one holds a slot while its point is scored and written, not just while it publishes.

In the storm, five times that is far past what two App Servers hold. Their slots fill, new points are refused, and about 64% of the points fail every second of the storm: 45.4% errors over the test, against a limit of 1%. The two GPUs are short too (about 135% of their slots in the storm). Fixing one alone doesn't help much: with 4 Triton instances the errors stay at 45.4%, because the App Servers are the first wall; with 8 App Servers and 2 Tritons they drop to 17.8%, because now the GPUs are.

The firmware update breaks the single node: 2,400,000 series is past twice its limit, so it refuses every write for 20 seconds. That is 33.3% errors over the test, and no alerts while it lasts.

An inline design can pass if it is sized for the storm everywhere: 8 App Servers, 4 Triton instances and 3 database nodes pass all three tests at about $4,416. The stream design costs about the same; what it buys is where the waiting happens. With a stream, the App Servers never wait on the model, and a classifier that is short in a storm makes alerts late instead of losing points. Set Triton to 2 instances in the reference and run the storm: there are no errors, but the alert latency climbs to about 7 s and the p95 is about 6.8 s, so the test fails on time, with every point still stored. Inline, the same shortfall loses the points.

The point is this: put the model call where waiting is cheap. A stream and a consumer in front of the classifier keep the ingest fast and turns a shortfall into lateness, not lost data. Then size the classifier for the storm you expect, with headroom, because a late alert in a storm is the one that matters. And size a time-series database by its series as well as its points.

## What the simulator leaves out

The classifier's quality is not simulated: whether a score finds a real anomaly, false alarms and missed ones are not part of this problem, and a point counts the same whatever its score. The alert itself (the pager, grouping and silencing alerts) is not simulated either; the alert latency here ends when the scored point is stored. A point enters at an App Server card that stands for the broker's bridge, because the simulator starts every request at a service; the broker's message rate and QoS don't touch the points. A storm and a bad firmware last hours in real life; the tests run 20 seconds of each. Each point is scored alone; real scorers often batch a sensor's window of points into one call. A real Flink job would call the model with Async I/O, so one slot keeps many calls in flight; the simulator holds a Stream Processor slot for each call, which is why the reference waits in worker threads. The GPU price is the registry's teaching figure for a g5.xlarge-class node, and the points per second and series limit are what you type on the card, not a measured node size, so the statement fixes them. Retention, downsampling and the price of storing the points are not simulated.

## Things to try

- Set Triton to 2 instances in the reference and run the storm. Watch the stream fill and the alert latency climb, with no errors.
- Set Triton to 3 instances and run the storm. It passes, but look at how busy the GPUs get, and ask how much stronger a storm it would survive.
- Set the database to 2 instances and run the firmware update. Each node holds 1,200,000 series and writes four times slower; at 1,500 points a second per node it still passes, with a p95 of about 50 ms. Then try it with 1 instance.
- Replace the Worker with an Apache Flink Stream Processor and find how many slots the storm needs. Compare its price with the workers'.
