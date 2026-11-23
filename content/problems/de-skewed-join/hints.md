## 1

Find the critical path. The job ends when its last task ends, and the biggest task is about 330 executor-seconds, possibly up to 1.5 times that. On a quiet night that already eats most of the 480 s. Now imagine the cloud stops that task and the executors only come back at 300 s. How much of it is left to run, and when can it finish?

## 2

Adding nodes or turning on "Speculative execution" doesn't make the biggest task any smaller: one task runs on one executor, and a copy of a huge task is just as huge. "Checkpoints" save part of a stopped task's work, but the hot task still needs most of its 330 s after the executors return. What shrinks it is "Salting (split hot tasks)": split the biggest tasks into enough pieces that each one is a few dozen seconds, and the job's length becomes the total work divided by the executors.

## 3

Apache Spark on Amazon EMR Batch Compute card with 6 instances, 4 executors per node (24 executors), "Salting (split hot tasks)" at 16 and "Checkpoints" on, wired with Query to an Amazon S3 Object Storage card and with Query to a Snowflake Data Warehouse card.
