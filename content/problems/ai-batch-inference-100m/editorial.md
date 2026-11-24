# Editorial

## Where the load goes

All numbers are in simulated time: the real 6-hour window is 600 s here, 36 times shorter. The job is 100 tasks of 300 executor-seconds, 30,000 executor-seconds in all. With at least 100 executors every task starts at once, and the job takes as long as its slowest task: 300 s times that task's noise (up to about 1.5), so about 450 s. That leaves about 150 s of slack, half a task.

An interruption costs time in two ways. First, it throws work away: a task stopped at 240 s and started again from zero ends at 240 + 300 = 540 s at the earliest, and later with noise; with checkpoints it keeps its last 10% mark and redoes only the rest, about 60 to 90 s. Second, it takes executors away: after losing half of them, a stopped task can only start again at once if there is a free executor left. If not, it waits for another task to finish (around 300 to 450 s) and then needs its own run on top. No amount of checkpointing rescues a task that cannot start until 400 s and still has 240 s to go.

So a spot cluster needs both: checkpoints, so a late interruption costs little work, and enough executors that the half that survives still holds all 100 tasks, so an early interruption costs no waiting. The spare half is cheap: a spot node costs $61.25 a month, 35% of an on-demand one, so 200 spot executors cost less than 100 on-demand ones.

## The reference design

An Apache Spark on Amazon EMR Batch Compute card with 50 instances, 4 executors per node (200 executors), "Spot share" 100% and "Checkpoints" on, wired with Query to an Amazon S3 Object Storage card. It costs about $3,065 per month: about $3,063 for the 50 spot nodes ($61.25 each) and $2 for Object Storage. On a quiet night it is only about 21% busy: half the cluster is a reserve for the night the cloud takes it back.

"A quiet night" finishes in about 455 s. "Half the spot executors are taken back early in the night" takes 100 executors at 60 s, and 50 running tasks stop. The 100 executors left hold the 50 tasks that kept running and the 50 that start again at once from their checkpoint, with 80 to 90% of their work still to do; the job ends at about 460 s. "Half the spot executors are taken back late in the night" stops 50 tasks at 240 s, when most of them are past their 60% to 80% checkpoint; they start again on free executors with one or two minutes of work left, and the job ends at about 420 s (a stopped task draws its noise again, so the slowest task of the night can even end sooner).

The way out of spot is honest too, and dearer: 25 on-demand nodes ("Spot share" 0%, 100 executors, no checkpoints) are untouched by both hidden tests and pass them in about 450 s and 420 s, for about $4,377 per month, 43% more than the reference. Half a cluster of spare spot nodes is cheaper insurance than a cluster nobody can take back.

For the cost challenge there is no target: the search's cheapest proof, 40 spot nodes with checkpoints, cost 80% of the reference ($2,452), which is not under the 80% line, and it passes the early interruption with little to spare.

## Why the first try breaks

A common first try is 28 nodes all on spot ("Spot share" 100%, 112 executors, enough for every task to start at once) without checkpoints, for about $1,717 per month. It passes "A quiet night" in the same 455 s as the reference: on a quiet night, more nodes don't help, because 100 tasks can't use more than 100 executors.

"Half the spot executors are taken back late in the night" breaks it. At 240 s, 50 tasks stop, most of them nearly done, and start again from zero on the free executors. Each needs another full 300 s or more, and the job ends at about 730 s, against a deadline of 600 s. The executors are there; the work was thrown away.

"Half the spot executors are taken back early in the night" breaks it too. At 60 s only 56 executors are left, and 50 of them still run the tasks that weren't stopped. Six stopped tasks start again at once; the other 44 wait until a running task finishes, around 300 s or later, and then need a whole run from zero. The job is not done when the run ends at 700 s.

The two fixes are not interchangeable. The same 28 nodes with checkpoints pass the late interruption (about 445 s) but still miss the early one (about 670 s): tasks keep their progress, but they wait for an executor. 50 nodes without checkpoints pass the early interruption (about 500 s) but miss the late one (about 695 s): every stopped task starts at once, but from zero. Cheap interruptible capacity needs both, checkpoints to keep the work and spare executors to keep the parallelism.

## What the simulator leaves out

Time is scaled 36 times shorter, as the statement says; the ratios between task time, interruption time and deadline are what matter. Spot capacity is priced at a flat 35% of on-demand here; real spot prices move with the hour, the zone and the instance type, and the tests decide how much capacity the cloud takes back and when. Real reclaims come with a short warning, hit whole nodes rather than an exact share, and often bring replacement nodes after a start-up delay; none of that is simulated. Writing a checkpoint is free here; a real one costs time and storage. The model runs as executor time only: loading it, memory per executor and GPU inference are not simulated, and the quality of the scores is not measured at all. The monthly cost assumes the cluster runs all month; a real nightly cluster can be shut down during the day.

## Things to try

- Turn "Checkpoints" off on the reference and run the hidden tests. Compare the late interruption with 240 s + 300 s.
- Drop the reference to 28 nodes with checkpoints on, and watch the early interruption: count the executors left against the 100 tasks.
- Raise "Executors per node" to 8 on 25 nodes. It passes, but it costs the same as 50 nodes: more executors per node are billed as a bigger machine. On a real m5.xlarge with 4 vCPUs, two executors per core would also make every task about twice as slow, which the simulator doesn't model.
- Set "Spot share" to 50% on the reference: each test now takes a quarter of the executors, and the bill rises to about $5,908, over the budget.
