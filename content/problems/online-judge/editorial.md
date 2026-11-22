# Editorial

## Where the load goes

At 40 submissions/s and about 2 seconds each, about 80 submissions are being judged at any moment. One Judge Worker instance runs 8 at a time, so a quiet afternoon needs 10 instances just to keep up, and 12 leaves a little headroom. The Submit API does almost nothing: it puts the submission on the Queue and returns. All the real cost is in the judges, and the latency that counts is the time until the verdict is stored. Whenever judges fall behind, that time is mostly time spent waiting in the Queue.

## The reference design

A Load Balancer spreads traffic over 2 Go Submit API servers. They publish to an SQS Queue with a max depth of 1,000,000. Judge Workers consume it, starting at 12 instances of 8 slots, inside an Autoscaling group with a minimum of 12, a maximum of 80, a target utilisation of 60% and a scale delay of 10 seconds. Each verdict is written to PostgreSQL.

This costs about $1,963 per month. On a quiet afternoon the p95 is about 5.7 s. When the contest starts, traffic ramps from 1 to 6 times over the minute, up to 240 submissions a second; the group keeps adding instances as utilisation climbs, and the p95 is about 21 s. When a judge machine crashes, the Judge Workers card is down for 20 seconds; submissions wait in the Queue, none are lost, and the p95 is about 23 s. Both hidden tests allow a p95 of up to 30 s.

## Why the first try breaks

A common first try is the same pipeline with a fixed 12 instances and no Autoscaling group. A quiet afternoon passes with a p95 near 6.5 s. During the contest, submissions soon arrive faster than 96 slots can judge them. Nothing is lost, because the Queue is deep, but the backlog grows every second, and the p95 reaches about 51 s.

## Things to try

- Raise the scale delay to 60 seconds and run the contest again. Where does the p95 land?
- Lower the maximum to 30 instances and find out whether the group still keeps up.
- Lower the minimum to 4 and compare the quiet afternoon's p95 and the monthly cost.
