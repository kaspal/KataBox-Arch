## 1

One database serves two very different readers: the app, which reads one small order at a time and needs a quick answer, and the export, which reads huge pages as fast as it can. Work out how many database connections each one keeps busy. What happens to the app's reads when the export's share triples?

## 2

The export only reads, and it doesn't need the very latest write. A read replica holds the same data as the primary, so it can take those reads off the primary's hands. Writes still have to go to the primary. Check what the "Read from" setting of an SQL Database does with reads when it has replicas.

## 3

Put an AWS ALB in front of 2 Go App Servers wired with Query to a PostgreSQL SQL Database (the primary). An Airflow Scheduler publishes to a RabbitMQ Queue, and a Celery Worker (4 instances, concurrency per instance 64, service time 2 ms) consumes it and is wired with Query to a second PostgreSQL SQL Database. Wire the primary to that second one with Replicate, and leave "Read from" on "Primary and replicas".
