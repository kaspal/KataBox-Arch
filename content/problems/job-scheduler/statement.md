# Distributed Job Scheduler

## The product

You run the service that runs everyone else's background work. Teams register jobs that must happen on a schedule: send the daily report, rebuild a search index, expire old sessions, renew a subscription. A scheduler fires each job at its time and hands it off. Nobody watches a screen while a job runs, but every job must run, and it must finish reasonably soon after it fires. A job that quietly disappears is the worst thing this service can do.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 100 jobs/s |
| p95 completion time | 10 s or less |
| Availability | 99.9% or more |
| Budget | $2,500 per month |
| Lost jobs | none |

Latency here is measured until the job has finished and its result is stored, not until it is handed off.

## The traffic

- **Run a scheduled job** (100%). A Scheduler (the cron) publishes each job to a Queue. Workers take jobs from the Queue and run them; each job takes about 100 ms of work. When a job is done, the Worker writes its result to a database.

The Scheduler does not slow down when the system behind it is busy. Jobs keep firing on time whether or not anyone is ready to run them.

## The tests

One test is visible. You can run it as often as you like:

- **Normal hour**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The worker pool is lost** and **Midnight**. Your design has to pass all three.
