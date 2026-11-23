# Editorial

## Where the load goes

Time here is scaled down: a job of about 2 s stands for a real one of about 15 minutes, and the 8 s limit for about an hour. The shape of the problem is the same at either scale.

Count GPU time, not jobs. At peak, 3 urgent jobs a second each hold a GPU for about 2 s, and 3 bulk jobs a second each hold one for about 6 s. So on a normal day about a quarter of the busy GPUs run urgent work and three quarters run bulk work. With 4 nodes (32 GPUs) the cluster is about 80% busy: that is what a shared cluster is for, and it keeps up.

Release day multiplies the urgent jobs by four. Urgent work alone now needs about as many GPUs as the whole normal load did, about 25 of them, and with the bulk jobs on top the cluster is asked for about 45 GPUs' worth of work. 32 GPUs can't do it, so a backlog grows every second the release push lasts. Which jobs wait in that backlog is decided by the queue.

## The reference design

A Web client calls 2 Go App Servers, which publish each job to a RabbitMQ Queue with "Live before bulk (priority)" on and a visibility timeout of 60 s. Five GPU nodes consume from it, each a Celery Worker with 8 Instances (one per GPU), "Concurrency per instance" 1 and a service time of 2,000 ms. This costs about $3,010 per month: $2,800 for the five GPU nodes, $140 for the App Servers and $70 for the Queue. On a normal day the p95 is about 4.2 s with no errors, and the nodes are about 60% to 70% busy.

Priority is the first half. Every urgent job goes ahead of every queued bulk job, so an urgent job waits only for the next GPU to come free, never for the pile. The bulk jobs take whatever GPUs are left, and on release day they are the ones that wait: they are accepted at once and run later, which is what bulk means. "Release day: four times the urgent jobs" has a p95 of about 5.3 s with no errors.

Headroom is the second half. Priority can't make GPUs appear: the urgent work by itself has to fit on the GPUs you still have when something breaks. "Release day, and a GPU node fails for 80 s" kills the first GPU node 20 seconds in, on release day. Four nodes are left, 32 GPUs for about 25 GPUs of urgent work, about 80% busy, and the p95 is about 6.0 s with no errors. The visibility timeout matters here: the jobs that were running on the failed node come back to the queue 60 s later and run again, so a handful of urgent jobs finish very late. They are far fewer than 5% of the jobs, so they don't move the p95. In real life, keep the visibility timeout longer than your longest job, or a slow job is handed out twice (the simulator only brings jobs back from a failed node).

There is no cost challenge for this problem. The search found a design at 80% of the cost only by making the first GPU card a single-GPU node, so that the node the test fails is tiny. In a real cluster any node can fail, so that design would not protect you.

## Why the first try breaks

A common first try is the same path with 4 GPU nodes and a plain first-in, first-out queue. It costs about $2,450 and passes "A normal working day" with a p95 of about 4.6 s, its nodes about 80% busy.

On release day the cluster gets about 45 GPUs' worth of work for 32 GPUs, and the queue grows every second. Each new urgent job joins the back of the line, behind every bulk job already waiting, so its wait grows with the pile: the p95 of each second passes 8 s within a few seconds and keeps climbing, and the test ends with a p95 of about 49 s. No job fails; urgent jobs just wait for bulk ones nobody is waiting on.

When a node also fails, the 24 GPUs left are even further behind, and the p95 goes past a minute.

Fixing only one half is not enough. With priority but still 4 nodes, release day passes (about 5.9 s), but when a node fails the urgent work alone, about 25 GPUs, no longer fits on the 24 that are left: p95 about 13.9 s. With more nodes but no priority, you need enough GPUs for all the work, urgent and bulk: 7 nodes ($4,130) still fail the node test at about 11.2 s, and 8 nodes pass but cost about $4,690, over the budget.

The point is this: on a shared GPU cluster the bulk work is meant to fill every spare GPU, so the cluster runs near full on purpose. Then the queue's order decides who waits, and urgent work has to go first. Size the cluster for the urgent work at its peak with a node lost, and let bulk work absorb the rest.

## What the simulator leaves out

The time scale: real jobs run for minutes or hours, and a real release day lasts longer than two minutes. Each GPU node is a Worker card, priced as the registry's Worker instances, not at real GPU prices, and the simulator does not stop you raising "Concurrency per instance" past one job per GPU. Running bulk jobs are never pre-empted (stopped to free a GPU for an urgent one), jobs never need more than one GPU, and there are no checkpoints, so a job on a failed node starts again from the beginning. GPU memory, the data each job loads, job placement across nodes and fair shares between teams are not simulated. The "Live before bulk (priority)" switch works on every Queue flavour here; RabbitMQ has message priorities, while with Amazon SQS you would use two queues.

## Things to try

- Turn "Live before bulk (priority)" off and run release day. Watch the p95 of each second climb.
- Remove one GPU node and run the node failure test. Compare the urgent work's GPUs with the GPUs left.
- Set the visibility timeout to 10 s and run the node failure test. The failed node's jobs come back sooner, but in real life every job longer than 10 s would also be handed out twice; the simulator doesn't show that.
