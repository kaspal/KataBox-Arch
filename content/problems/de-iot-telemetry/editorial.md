# Editorial

## Where the load goes

25,000 meters send a reading every 10 seconds: 2,500 readings a second, each one write to the Time-Series Database. One node takes 4,000 points a second, so on a normal day one node is enough, at about 62% of its capacity. The 25,000 MQTT connections are easy for the IoT Broker: two EMQX instances hold up to 2,000,000.

The hidden tests push on the two limits of the database node. When the fleet reconnects after a network outage, the readings arrive 4 times as fast for 20 seconds (from 20 s in): 10,000 a second, two and a half times what one node writes. When the firmware update adds a label, the 250,000 series become 2,500,000 for 20 seconds. One node indexes 1,000,000; past 2,000,000 it refuses the writes.

So the questions are: where do readings wait when they arrive faster than the database writes, and how many nodes do the series need?

## The reference design

The meters keep their MQTT connections on an EMQX IoT Broker with the default 2 instances. Readings enter at a Go App Server (2 instances), which publishes each one to an Apache Kafka Stream with 192 partitions. Six Celery Workers (concurrency 32 per instance, service time 2 ms) consume the stream and write to an InfluxDB Time-Series Database with 3 instances at 4,000 points per second each. It costs about $1,405 per month: $552 for the database, $420 for the workers, $153 for Kafka, and $140 each for the broker and the App Servers. A normal day has a p95 of about 56 ms from the reading to the database, and no errors.

The stream does the work in "The fleet reconnects after a network outage". The App Server only publishes, so it answers quickly whatever the database does. The workers take readings off the stream as fast as their slots allow, and that pace stays under what the three nodes write, so nothing is refused: the extra readings wait in Kafka, and the workers catch up a few seconds after the burst ends. Freshness slips while they wait. This test allows a p95 of 10 s, and the design gets about 4.8 s, with the worst second at about 5.2 s and no errors.

"A firmware update multiplies the series by 10" is where the series budget matters. With 3 instances, each node holds 2,500,000 / 3 ≈ 833,000 series during the explosion, still under its limit of 1,000,000. Writes keep their normal speed, and the p95 stays at about 56 ms with no errors. Planning the series per node with room for a ten-fold surprise is what decides the instance count here, more than the points per second.

The cost challenge search found no design under 80% of the reference's cost (the best it reached was 85%), so this problem has no cost challenge.

## Why the first try breaks

A common first try is the same IoT Broker and App Servers, with the App Server writing straight to one InfluxDB node at 4,000 points per second: no stream and no workers. It costs about $464 and passes "A normal day" with a p95 of about 52 ms; the database runs at about 75% busy.

When the fleet reconnects, 10,000 readings a second hit a node that writes 4,000. Its queue fills in a moment and it refuses the rest: about two thirds of the readings fail during the burst, 44.2% over the whole test. Adding database nodes without a stream doesn't fix it either. With 3 nodes and no stream, the App Servers wait on every write themselves, their slots fill up, and 34.2% of the readings still fail.

The firmware update is worse for one node: 2,500,000 series on a node with a limit of 1,000,000 is past twice the limit, so it refuses every write for 20 seconds. That is 32.7% errors over the test.

The point is this: a burst you can wait out belongs in a stream, not in the database's queue. A stream keeps the readings and the workers deliver them at a pace the database can take, so a burst costs a few seconds of freshness instead of lost readings. And size a time-series database by its series as well as its points: count the series per node, and leave room for the label someone will add by mistake.

## What the simulator leaves out

The network outage itself is not simulated: the meters stay connected the whole time, and the test only replays the upload of buffered readings as 4 times the normal rate for 20 seconds. The real reconnect storm on the broker (meters dropping and reconnecting together) is not part of this problem. A reading here enters at an App Server card that stands for the broker's bridge, because the simulator starts every request at a service; the broker's message rate and QoS don't touch the readings. Each reading is one point; real meters often batch many points in one write. Points per second and the series limit are what you type on the card, not a measured node size, so the statement fixes them. Retention, downsampling, compression and disk size are not simulated, and neither is the price of storing the points.

## Things to try

- Set the database to 2 instances in the reference and run the firmware update. Each node holds 1,250,000 series, writes four times slower, and the stream fills: the p95 climbs to about 5.9 s.
- Remove the stream and give the database 3 instances. Run the reconnect burst and watch the App Servers, not the database.
- Give the workers fewer slots and run the reconnect burst. Compare how long the stream takes to drain with the 10 s limit.
