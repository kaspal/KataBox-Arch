## 1

Count the points a second in a storm, and how long each one waits for its score. Whoever calls the classifier holds a slot for that whole wait. Then count the series each database node holds, and multiply by 8.

## 2

If the App Server scores each point inline, its slots are held while the model works, so a storm fills them and points are refused. Put a stream after the App Server and let a consumer call the classifier and write to the database: the App Server only publishes. The consumer mostly waits, so give it threads that are cheap to hold while they wait. Then size the classifier for the storm, not the normal day: a GPU that is short in a storm makes every alert late. For the series, more database instances each hold a smaller share; plan them so that 8 times the series still stays under each node's limit.

## 3

Send the points from the Go App Server (2 instances) to an Apache Kafka Stream with 384 partitions. A Celery Worker (6 instances, Concurrency 64 per instance, Service time 1 ms) consumes it, calls NVIDIA Triton with 4 instances, and writes to an InfluxDB Time-Series Database with 3 instances at 20,000 points per second each. Keep the sensors connected to an EMQX IoT Broker with the default 2 instances.
