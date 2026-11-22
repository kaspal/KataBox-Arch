# Editorial

## Where the load goes

At 100 jobs/s and about 100 ms each, the steady work keeps about 10 Worker slots busy at any moment. That is small. The difficulty is not the average but the moments when demand and capacity stop matching: when jobs arrive faster than Workers can run them, or when there are no Workers at all. The Scheduler never waits, so every job it fires has to be stored somewhere until a Worker is free. The Queue is that place, and its depth decides whether a bad minute means late jobs or lost jobs. Late is allowed. Lost is not.

## The reference design

The Scheduler publishes to an SQS Queue with a max depth of 1,000,000. Celery Workers consume it, inside an Autoscaling group with a minimum of 2, a maximum of 24, a target utilisation of 60% and a scale delay of 10 seconds. Each Worker writes its result to PostgreSQL, which handles 100 small writes a second with ease.

This costs about $759 per month. On a normal hour the p95 completion time is about 0.3 s. When the whole worker pool goes down for 20 seconds, jobs pile up in the Queue, none are lost, and the p95 rises to about 19 s while the backlog drains. At midnight, when 10 times the jobs arrive for 10 seconds, the Queue absorbs the burst, the group scales up, and the p95 reaches about 24 s. Both hidden tests allow a p95 of up to 40 s.

## Why the first try breaks

A common first try uses a RabbitMQ Queue with a max depth of 1,000 and 2 fixed Workers. On a normal hour it is fine. When the pool is lost for 20 seconds starting at 20 s, about 2,000 jobs arrive with nobody to take them. The Queue fills at 1,000, and about 980 jobs are refused and lost. At midnight the 2 Workers cannot keep up with 1,000 jobs a second, the Queue fills again, and about 7,800 jobs are lost.

## Things to try

- Keep the Autoscaling group but set the max depth back to 1,000. Which test still loses jobs?
- Raise the scale delay to 30 seconds and watch the p95 at midnight.
- Lower the maximum to 8 instances and see how long the midnight backlog takes to drain.
