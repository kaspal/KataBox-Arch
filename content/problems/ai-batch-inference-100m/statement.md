# Score 100 Million Items Overnight

## The product

A shopping site ranks its whole catalogue with a model: every night, each of its 100 million items gets a fresh relevance score, and the morning's search and recommendation pages read those scores. The items sit in Object Storage in 100 shards of 1 million items each. A Batch Compute cluster (Apache Spark on Amazon EMR) runs the job: one task per shard reads the shard, runs the scoring model on the executors, and writes the scores back to Object Storage. The model is a small file each executor loads, so the work is executor time, not calls to a model service.

The cluster runs on spot instances, because they are much cheaper. The price of spot is that the cloud can take nodes back at any moment, and every task running on them stops. The scores must still be ready by morning.

**Scaled time.** The real job has a window of 6 hours. The simulator runs minutes, so time is scaled down 36 times: the window is 600 simulated seconds, and 1 simulated second stands for 36 real seconds. A shard needs 300 executor-seconds of work in the simulator (about 3 real hours on one executor, roughly 90 items a second). Every number below is in simulated time.

## What you must meet

| Goal | Target |
| --- | --- |
| Job | 100 tasks of 300 executor-seconds each, starting at 0 s |
| Deadline | the whole job finishes within 600 s of its start |
| Errors | 0.1% or less |
| Budget | $12,000 per month |

The monthly cost is what the cluster would cost if it ran all month, the same rule as every other problem (a Batch Compute node costs $140 a month, whatever its Spot share). Prices are the registry's teaching prices.

## The traffic

- **Score tonight's items** (a job). It starts at 0 s and splits into 100 equal tasks, one per shard. Each task waits for a free executor, holds it for its work (each run varies a little, up to about 1.5 times at worst, plus the task overhead), then reads from and writes to Object Storage. The job finishes when its last task does.

A Batch Compute node is an m5.xlarge-class machine with 4 vCPUs, so keep "Executors per node" at 4: the number of nodes is what you size. Parallel tasks = nodes × executors per node.

When the cloud takes spot capacity back, a share of the cluster's executors disappears until the capacity returns, and the same share of the running tasks stop and start again. Without checkpoints a stopped task starts again from zero. With "Checkpoints" on the Batch Compute card, it starts again from its last checkpoint (one every 10% of the task). In the tests, the share taken back does not depend on the card's "Spot share": treat the whole cluster as spot.

## The tests

One test is visible. You can run it as often as you like:

- **A quiet night**: the job runs with no interruptions.

Two more tests are hidden, and you see only their names: **Half the executors are taken back early in the night** and **Half the executors are taken back late in the night**. Your design has to pass all three.
