# Editorial

## Where the load goes

All numbers are in simulated time: the real 4-hour window is 480 s here, 30 times shorter. The job is 2,700 executor-seconds of work, so 24 executors could do it in about 2,700 ÷ 24 ≈ 113 s, plus the task overhead (2,000 tasks × 0.2 s ÷ 24 ≈ 17 s). That is the job's length only if the work spreads evenly, and here it doesn't. The task sizes follow the sellers' sales (a Zipf skew of 1.0 over 2,000 sellers): the marketplace's own store is 2,700 ÷ 8.18 ≈ 330 executor-seconds in a single task, 12% of the whole job, and the next sellers are 165, 110 and 83 s.

One task runs on one executor, so the job can't finish before its biggest task: about 330 s times that task's noise, plus the time it waited in the queue for its first executor. The other 23 executors finish everything else in about two minutes and then sit idle. On a quiet night that usually fits in 480 s, but with little to spare (a slow draw of up to 1.5 times is about 495 s on its own), and the hot task is exposed for most of the night: any interruption that stops it costs its whole length, or most of it, all over again.

"Salting (split hot tasks)" changes the shape of the job. With salting at 16, each of the 20 biggest tasks (the top 1%) is split into 16 equal pieces: the store's task becomes 16 pieces of about 21 s, and no task in the job is bigger than about 21 s. The job's length is now close to the total work divided by the executors, and an interruption only costs the pieces that were running.

## The reference design

An Apache Spark on Amazon EMR Batch Compute card with 10 instances and 4 executors per node (40 executors), "Spot share" 100%, "Salting (split hot tasks)" at 16 and "Checkpoints" on, wired with Query to an Amazon S3 Object Storage card (the orders) and with Query to a Snowflake Data Warehouse card (the report). It costs about $867 per month: $490 for the 10 spot nodes ($49 each, less than four on-demand nodes would cost), $3 for Object Storage and $373 for Snowflake, which bills only while it is awake to take the writes.

"A regular night" finishes in about 120 s, with six minutes to spare. "All spot executors are taken back for two hours, early in the night" stops every running task at 60 s and leaves the cluster empty until 300 s; the salted tasks are small, so the job picks up where it was and ends at about 365 s. "All spot executors are taken back for 15 minutes, late in the night" comes at 270 s, long after the job finished, and changes nothing.

Why 10 nodes and not 6? Once salting has made the job even, more executors do shorten it, and spot executors are cheap. With 6 nodes the design also passes, for about $783, but it ends the early test at about 415 s, a thin margin on a 480 s deadline. The four extra spot nodes cost $196 a month (the total rises only about $84, because Snowflake goes back to sleep sooner) and keep the early test about two minutes clear of the deadline.

"Checkpoints" make little difference to the reference here (without them it ends the early test at about 380 s): a stopped piece of 21 s loses at most 21 s. They are cheap insurance, and they are what an unsalted job needs most, as the next section shows.

For the cost challenge, a proof design comes in at 58% of the reference's cost, under the 60% target. It keeps salting and checkpoints on 5 spot nodes and writes the report to a ClickHouse Data Warehouse instead of Snowflake, about $250 a month here against $373. What it gives up is the margin: it ends the early interruption at about 450 s.

## Why the first try breaks

A common first try sees one task running long and reaches for the two usual fixes: "Speculative execution" on and nodes to spare. With 6 nodes all on spot and speculation it costs about $1,004 per month (Snowflake stays awake longer, while the long task finishes) and passes "A regular night" in about 415 s: the hot task alone sets the time, and the cluster is about 28% busy over the test.

"All spot executors are taken back for 15 minutes, late in the night" breaks it. At 270 s the hot task is about two thirds of the way through its run, and it starts again from zero when the executors return at 300 s. It needs its whole run again and ends at about 595 s, against a deadline of 480 s.

"All spot executors are taken back for two hours, early in the night" breaks it too, and here checkpoints don't save it. At 60 s the hot task has done less than a fifth of its work. When the executors come back at 300 s it still has about 300 s to go, so it ends at about 535 s whatever else the cluster does.

None of the usual fixes touches the cause:

- **Speculation** copies a task that runs much longer than the typical one, but the copy has the same 330 s of work. It only helps with a slow draw: without it the first try ends the quiet night at about 435 s instead of 415 s, and it fails the same hidden tests.
- **More nodes** finish the small tasks sooner but can't split the big one. With 12 nodes (and checkpoints) the quiet night ends at about 405 s, the early interruption still at about 565 s, and the bill grows by about $263 a month.
- **Checkpoints** without salting just pass the late interruption (about 475 s: the hot task keeps its 60% mark) but still miss the early one (about 535 s), because the task had little progress to keep.
- **Leaving spot** works, at a price: the first try with "Spot share" 0% is never interrupted and passes every test, for about $1,550 per month, 79% more than the reference. Every night still waits about 415 s on one executor, and a slow draw of the hot task can miss the deadline on a quiet night.

Only salting removes the giant task, and with it the hours the job spends waiting on one executor.

## What the simulator leaves out

Time is scaled 30 times shorter, as the statement says; the ratios between task sizes, interruptions and the deadline are what matter. The job's skew is fixed: every test runs the same data, so the tests vary when the cloud takes capacity back, not how hot the key is. Salting is a setting here; in a real join it means adding a random salt to the hot key on the big side, copying the matching rows of the small side once per salt value, and combining the partial results again, which costs extra shuffle and memory that the simulator doesn't charge (shuffle traffic and memory spills are not simulated). The simulator splits exactly the biggest 1% of tasks; a real job has to find its hot keys first, from table statistics or with adaptive query execution. Spot capacity is priced at a flat 35% of on-demand here; real spot prices move with the hour, the zone and the instance type, and the tests decide how much capacity the cloud takes back and when. Real reclaims come with a short warning and often bring replacement nodes after a start-up delay, and writing a checkpoint is free here. The monthly cost assumes the cluster runs all month; a real nightly cluster can be shut down during the day. Whether the report's numbers are right is not judged.

## Things to try

- Set "Salting (split hot tasks)" to 1 on the reference and run the hidden tests: the job's length jumps from about 120 s to the hot task's length (about 410 s), and the early interruption ends at about 570 s.
- Try salting at 4 instead of 16: the hot task becomes 4 pieces of about 80 s. Compare its early-interruption time (about 435 s) with the reference's.
- Double the first try to 12 nodes and compare the quiet night's time with 6 nodes: it barely moves.
