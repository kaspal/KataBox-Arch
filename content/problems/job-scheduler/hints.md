## 1

When nobody is available to run a job, where does that job wait, and how many jobs can wait there before some are turned away?

## 2

Use a deep, durable queue as the buffer between the scheduler and the workers, and let the worker pool grow with the backlog instead of keeping a fixed size.

## 3

Use an SQS Queue with a very large max depth, so it never refuses a job, and put the Workers inside an Autoscaling group with a small minimum, a generous maximum, a moderate target utilisation and a short scale delay. A PostgreSQL database is plenty for the results.
