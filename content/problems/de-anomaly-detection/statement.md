# Turbine Anomaly Alerts

## The product

You run the monitoring backend for a wind energy company: 1,500 turbines with 20 sensors each, 30,000 sensors in all (vibration, bearing temperature, blade pitch and so on). Each sensor keeps an MQTT connection open to your IoT Broker and sends a point every 10 seconds. Every point is scored by an anomaly classifier your team trained, and stored with its score in a Time-Series Database; the on-call engineers get an alert from the stored score. A failing bearing has to raise its alert within 1 second of the point. Two things go wrong in real life. In a storm, the sensors switch to fast sampling and send 5 times as many points, exactly when anomalies matter most. And now and then a firmware update adds a label it shouldn't, and the number of series jumps.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 3,000 points/s and 30,000 connected sensors |
| p95 latency | 1 s or less, from the point to its score in the database (the alert latency) |
| Error rate | 1% or less |
| Availability | 99.9% or more |
| Connected sensors | 95% or more in every second |
| Budget | $7,600 per month |

## The traffic

There are two request types:

- **Sensors connected over MQTT**. 30,000 sensors hold an MQTT connection to an IoT Broker, connected from the start. Your backend sends them nothing in this problem.
- **Score and store a point** (100% of the points, 3,000 a second). In the simulator, a point enters at an App Server: think of it as the bridge that takes each publish off the broker. From there it can go through an optional stream to a consumer (a Stream Processor or a worker). Whoever handles the point calls a Model Inference card with Model type set to Classifier, then writes the point and its score to a Time-Series Database. Without a stream, the App Server makes both calls itself. Latency is the time from the point to its write in the database (completion), not the App Server's answer.

The classifier reads a short window of features for each point: about 30 tokens in (95th percentile 60), and one score out. It runs on your own GPUs with NVIDIA Triton; keep the flavour's settings (Model type Classifier, 64 Batch slots per GPU) and choose the Instances. The database tracks 300,000 series: 10 per sensor.

Your team runs one node size for the Time-Series Database. Keep these two settings on every Time-Series Database card, because that is what the node you pay for can do:

- **Points per second**: 20,000 per instance.
- **Series limit**: 1,000,000 per instance (the default). The series are spread evenly over the instances. Above the limit a node writes four times slower, and above twice the limit it refuses the writes.

There are no outside services in this problem.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A storm: the sensors send 5 times as many points** and **A firmware update multiplies the series by 8**. Your design has to pass all three. A test runs 60 seconds, so a storm or a bad firmware lasts 20 seconds in it instead of hours.
