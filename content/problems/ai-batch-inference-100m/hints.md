## 1

Work out the timeline of one task. A task takes about 300 s, and the deadline is 600 s. If a task is stopped 240 s into its run and has to start from zero, when does it finish at the earliest? Now think about a task stopped 60 s in, when half the executors are gone and every remaining one is busy.

## 2

Two things go wrong, and they need two different fixes. A late interruption throws away almost a whole task's work: "Checkpoints" on the Batch Compute card keep it, so a stopped task only redoes its last 10%. An early interruption takes half the executors for the rest of the night: if what is left is fewer than the 100 tasks, some tasks wait for another to finish and then need a whole run of their own. Size the cluster so that half of it still holds every task. Spare spot nodes are cheap: at "Spot share" 100% a node costs $61.25 a month, about a third of an on-demand one.

## 3

Apache Spark on Amazon EMR Batch Compute card with 50 instances, 4 executors per node (200 executors), "Spot share" 100% and "Checkpoints" on, wired with Query to an Amazon S3 Object Storage card.
