# Logs & Metrics Pipeline

## The product

You run the logging platform for a company with hundreds of services. Each service gathers its log lines and ships them to you in batches. Your pipeline keeps every batch in a cheap long-term archive, and engineers search the logs when something goes wrong. The flow is heavy and steady, and it gets heavier exactly when things break: an incident makes every service log more. A batch that never reaches the archive is a hole in the record of what happened, so none may be lost.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 1,000 requests/s |
| p95 latency (batches measured until archived) | 2 s or less |
| Availability | 99.9% or more |
| Budget | $6,000 per month |
| Lost messages | None |

## The traffic

Services reach a Collector, an App Server, usually through a Load Balancer.

- **Ship a batch of log lines** (99%). Each batch is about 256 KB. The Collector publishes the batch to a stream and answers at once. A Worker takes it from the stream and writes it to an Object Store, the archive. Latency is measured until the batch is archived, not until the Collector answers.
- **Search the logs** (1%). An engineer runs a query, and the Collector asks a Search index for the matching lines.

No outside providers are involved. If the pipeline stops shipping, the batches waiting in the stream still count: when a test ends, any batch not yet archived counts at the age it has reached by then.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **An incident triples the logs** and **A consumer dies and the group rebalances**. Your design has to pass all three.
