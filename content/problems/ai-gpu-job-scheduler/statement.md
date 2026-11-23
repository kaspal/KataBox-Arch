# Schedule Jobs on a Shared GPU Cluster

## The product

Your ML platform team runs one shared GPU cluster for the whole company. Researchers submit jobs to it through an internal API: urgent eval runs that someone is waiting on (a release candidate must be checked before it ships), and bulk jobs that nobody waits on (hyperparameter sweeps, scheduled fine-tunes, nightly benchmark runs). GPUs are expensive, so the cluster is meant to stay busy: bulk work fills every GPU the urgent work doesn't need. What your users mind is an urgent job stuck behind a pile of bulk ones.

**Scaled time.** Real jobs run for minutes to hours. The simulator runs two minutes, so time is scaled down: a job that takes about 2 s on a GPU here stands for one of about 15 minutes, and the 8 s limit stands for "done within about an hour of being submitted".

## What you must meet

| Goal | Target |
| --- | --- |
| Peak submissions | 6 jobs/s (3 urgent, 3 bulk) |
| p95 latency (urgent: from submitted to done; bulk: until the submission is accepted) | 8 s or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Budget | $4,500 per month |

## The traffic

- **Submit an urgent eval job** (50%). The request reaches your App Servers, which put the job on a Queue. A Worker takes it from the Queue and runs it on one GPU. The time measured is from the submission until the job is done.
- **Submit a bulk job (sweep or scheduled fine-tune)** (50%). Same path, but a bulk job runs three times as long on its GPU. Nobody waits on it: the time measured is only until the submission is accepted (the Queue stores it), so the p95 limit is in practice decided by the urgent jobs (on a normal day, about their slowest 10%). Bulk jobs are marked as bulk work, so a Queue with "Live before bulk (priority)" on hands out every urgent job first.

An urgent job runs for about 2 s on one GPU at the median, with a slow tail of about 8 s at p99; a bulk job runs about 6 s at the median. The cluster is built from GPU nodes of 8 GPUs each, and a GPU runs one job at a time. Draw each GPU node as one Worker card with 8 Instances (one per GPU), "Concurrency per instance" 1 and "Service time" 2,000 ms; the simulator does not stop you raising the concurrency, so keep it at 1. A job keeps its GPU until it ends.

## The tests

One test is visible. You can run it as often as you like:

- **A normal working day**: the traffic above, at peak, for two minutes.

Two more tests are hidden, and you see only their names: **Release day: four times the urgent jobs** and **Release day, and a GPU node fails for 80 s**. Your design has to pass all three.
