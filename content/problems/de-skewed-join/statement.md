# A Join with One Giant Key

## The product

A marketplace builds its sellers' daily sales report every night. A Batch Compute job (Apache Spark on Amazon EMR) joins the day's orders, stored in Object Storage, with the sellers table, groups them by seller, and writes the result to a Data Warehouse (Snowflake). Analysts and sellers open the report first thing in the morning, so it has to be ready by then.

The data is skewed. Most of the 2,000 sellers sell a little, a few sell a lot, and the marketplace's own store sells so much that it holds about 12% of all orders. The join splits the work by seller, one task per seller partition, so that one key makes one task far bigger than all the others.

The cluster runs on spot instances, because they are much cheaper. The cloud can take them back at any moment, and every task running on them stops.

**Scaled time.** The real job has a window of 4 hours. The simulator runs minutes, so time is scaled down 30 times: the window is 480 simulated seconds, and 1 simulated second stands for 30 real seconds. The whole join is 2,700 executor-seconds of work in the simulator (about 22 real executor-hours). Every number below is in simulated time.

## What you must meet

| Goal | Target |
| --- | --- |
| Job | 2,700 executor-seconds in 2,000 skewed tasks, starting at 0 s |
| Deadline | the whole job finishes within 480 s of its start |
| Errors | 0.1% or less |
| Budget | $1,650 per month |

The monthly cost is what the cards would cost if they ran all month, the same rule as every other problem (a Batch Compute node costs $140 a month on demand, and the share of its executors set by "Spot share" runs on spot capacity at 35% of that, so a node at 100% costs $49; Snowflake bills only while it is awake). Prices are the registry's teaching prices.

## The traffic

- **Join tonight's orders with sellers** (a job). It starts at 0 s and splits into 2,000 tasks, one per seller partition. The task sizes follow the sellers' sales: the biggest task, the marketplace's own store, is about 330 executor-seconds of work; the second is about half that; most tasks take under a second. Each task waits for a free executor, holds it for its work (each run varies a little, up to about 1.5 times at worst, plus the task overhead), then reads from Object Storage and writes to the Data Warehouse. The job finishes when its last task does.

A Batch Compute node is an m5.xlarge-class machine with 4 vCPUs, so keep "Executors per node" at 4: the number of nodes is what you size. More executors per node are billed as a bigger machine, in proportion. Parallel tasks = nodes × executors per node. One task runs on one executor: more executors never make a single task shorter.

The Batch Compute card has three settings for this kind of job:

- **Salting (split hot tasks)**: the join adds a random "salt" to the hottest keys, so each of the biggest 1% of tasks is split into this many equal pieces that run on different executors.
- **Speculative execution**: near the end of the job (95% of tasks done), a task running much longer than the typical one gets a second copy, and the first copy to finish wins.
- **Checkpoints**: a task stopped by a spot interruption starts again from its last checkpoint (one every 10% of the task) instead of from zero.

When the cloud takes spot capacity back, that share of the card's spot executors disappears until the capacity returns, and the same share of its running tasks stop and start again. On-demand executors are never taken back: at "Spot share" 100% the whole cluster is exposed, at 0% nothing is.

## The tests

One test is visible. You can run it as often as you like:

- **A regular night**: the job runs with no interruptions.

Two more tests are hidden, and you see only their names: **All spot executors are taken back for two hours, early in the night** and **All spot executors are taken back for 15 minutes, late in the night**. Your design has to pass all three.
