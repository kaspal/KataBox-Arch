# Online Judge

## The product

You run an online judge for programming practice and contests. A user submits a solution, the judge compiles it, runs it against a set of tests in a sandbox, and stores a verdict: accepted, wrong answer, time limit exceeded. Running a submission is slow work, a couple of seconds each. Users accept a short wait for their verdict, but not a long one, and certainly not a lost submission. Most of the day is quiet. Contests are not.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 40 submissions/s |
| p95 completion time | 20 s or less |
| Availability | 99.9% or more |
| Budget | $3,000 per month |

Latency here is measured until the verdict is stored, not until the submission is accepted.

## The traffic

- **Judge a submission** (100%). The request passes an edge component such as a Load Balancer, then reaches the Submit API on an App Server, which puts the submission on a Queue. Judge Workers take submissions from the Queue and run them. Each submission takes about 2 seconds, and one Judge Worker instance runs 8 at a time. When it is done, the Worker writes the verdict to the database.

Submissions keep arriving whether or not a judge is free. Whatever cannot run yet has to wait somewhere.

## The tests

One test is visible. You can run it as often as you like:

- **A quiet afternoon**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The contest starts** and **A judge machine crashes**. Your design has to pass all three.
