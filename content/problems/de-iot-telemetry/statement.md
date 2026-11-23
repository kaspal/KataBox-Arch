# IoT Fleet Telemetry

## The product

You run the backend for 25,000 smart energy meters. Each meter keeps an MQTT connection open to your IoT Broker and sends a reading every 10 seconds. The readings land in a Time-Series Database, and the operations team watches them on live dashboards: a reading should show up within 2 seconds. Meters are cheap and their networks are not, so two things happen in real life. A whole area loses its mobile network, and when it comes back every meter uploads the readings it kept while it was offline. And now and then a firmware update adds a label to the readings that it shouldn't, and the number of series jumps.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 2,500 readings/s and 25,000 connected meters |
| p95 latency | 2 s or less, from the reading to the database |
| Error rate | 1% or less |
| Availability | 99.9% or more |
| Connected meters | 95% or more in every second |
| Budget | $3,000 per month |

## The traffic

There are two request types:

- **Devices connected over MQTT**. 25,000 meters hold an MQTT connection to an IoT Broker, connected from the start. Your backend sends them nothing in this problem.
- **Write a reading** (100% of the readings, 2,500 a second). In the simulator, a reading enters at an App Server: think of it as the bridge that takes each publish off the broker. From there it can go through an optional stream to a worker, and it ends as one write to a Time-Series Database. Without a stream, the App Server makes the write itself. Latency is the time from the reading to its write in the database (completion), not the App Server's answer.

The database tracks 250,000 series: 10 per meter (power, voltage, temperature and so on).

Your team runs one node size for the Time-Series Database. Keep these two settings on every Time-Series Database card, because that is what the node you pay for can do:

- **Points per second**: 4,000 per instance. Above it, writes wait in the node's queue, and when the queue is full they are refused.
- **Series limit**: 1,000,000 per instance (the default). The series are spread evenly over the instances. Above the limit a node writes four times slower, and above twice the limit it refuses the writes.

There are no outside services in this problem.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The fleet reconnects after a network outage** and **A firmware update multiplies the series by 10**. Your design has to pass all three.
