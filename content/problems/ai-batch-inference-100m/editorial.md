# Editorial

## Where the load goes

All numbers are in simulated time: the real 6-hour window is 600 s here, 36 times shorter. The job is 100 tasks of 300 executor-seconds, 30,000 executor-seconds in all. With at least 100 executors every task starts at once, and the job takes as long as its slowest task: 300 s times that task's noise (up to about 1.5), so about 450 s. That leaves about 150 s of slack, half a task.

An interruption costs time in two ways. First, it throws work away: a task stopped at 240 s and started again from zero ends at 240 + 300 = 540 s at the earliest, and later with noise; with checkpoints it keeps its last 10% mark and redoes only the rest, about 60 to 90 s. Second, it takes executors away: after losing half of them, a stopped task can only start again at once if there is a free executor left. If not, it waits for another task to finish (around 300 to 450 s) and then needs its own run on top. No amount of checkpointing rescues a task that cannot start until 400 s and still has 240 s to go.

So the cluster needs both: checkpoints, so a late interruption costs little work, and enough executors that the half that survives still holds all 100 tasks, so an early interruption costs no waiting.

## The reference design

An Apache Spark on Amazon EMR Batch Compute card with 50 instances and 4 executors per node (200 executors) and "Checkpoints" on, wired with Query to an Amazon S3 Object Storage card. It costs about $7,002 per month: $7,000 for the 50 nodes and $2 for Object Storage. On a quiet night it is only about 21% busy: half the cluster is a reserve for the night the cloud takes it back.

"A quiet night" finishes in about 455 s. "Half the executors are taken back early in the night" takes 100 executors at 60 s, and 50 running tasks stop. The 100 executors left hold the 50 tasks that kept running and the 50 that start again at once from their checkpoint, with 80 to 90% of their work still to do; the job ends at about 460 s. "Half the executors are taken back late in the night" stops 50 tasks at 240 s, when most of them are past their 60% to 80% checkpoint; they start again on free executors with one or two minutes of work left, and the job ends at about 420 s (a stopped task draws its noise again, so the slowest task of the night can even end sooner).

For the cost challenge there is no target: the search's cheapest proof, 40 nodes with checkpoints, cost 80% of the reference, which is not under the 80% line, and it passes the early interruption with little to spare.

## Why the first try breaks

A common first try is 28 nodes (112 executors, enough for every task to start at once) without checkpoints, for about $3,922 per month. It passes "A quiet night" in the same 455 s as the reference: on a quiet night, more nodes don't help, because 100 tasks can't use more than 100 executors.

"Half the executors are taken back late in the night" breaks it. At 240 s, 50 tasks stop, most of them nearly done, and start again from zero on the free executors. Each needs another full 300 s or more, and the job ends at about 730 s, against a deadline of 600 s. The executors are there; the work was thrown away.

"Half the executors are taken back early in the night" breaks it too. At 60 s only 56 executors are left, and 50 of them still run the tasks that weren't stopped. Six stopped tasks start again at once; the other 44 wait until a running task finishes, around 300 s or later, and then need a whole run from zero. The job is not done when the run ends at 700 s.

The two fixes are not interchangeable. The same 28 nodes with checkpoints pass the late interruption (about 445 s) but still miss the early one (about 665 s): tasks keep their progress, but they wait for an executor. 50 nodes without checkpoints pass the early interruption (about 495 s) but miss the late one (about 695 s): every stopped task starts at once, but from zero. Cheap interruptible capacity needs both, checkpoints to keep the work and spare executors to keep the parallelism.

## What the simulator leaves out

Time is scaled 36 times shorter, as the statement says; the ratios between task time, interruption time and deadline are what matter. The "Spot share" prop changes nothing in the simulator: a node costs the same teaching price whatever it says, so the spot discount, which is the reason to use spot at all, is not in the bill, and the tests decide how much capacity the cloud takes back. Real reclaims come with a short warning, hit whole nodes rather than an exact share, and often bring replacement nodes after a start-up delay; none of that is simulated. Writing a checkpoint is free here; a real one costs time and storage. The model runs as executor time only: loading it, memory per executor and GPU inference are not simulated, and the quality of the scores is not measured at all. The monthly cost assumes the cluster runs all month; a real nightly cluster can be shut down during the day.

## Things to try

- Turn "Checkpoints" off on the reference and run the hidden tests. Compare the late interruption with 240 s + 300 s.
- Drop the reference to 28 nodes with checkpoints on, and watch the early interruption: count the executors left against the 100 tasks.
- Raise "Executors per node" to 8 on 25 nodes. The simulator lets it pass at half the price, but an m5.xlarge has only 4 vCPUs: in a real cluster, two executors per core would make every task about twice as slow.
