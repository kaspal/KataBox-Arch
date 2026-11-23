# Editorial

## Where the load goes

At 1,000 players a second, the work is the matchers'. Each player holds a matcher slot for about 200 ms of matching, so 1,000 × 0.2 s keeps about 200 slots busy, before the write to DynamoDB. 4 Celery instances at a concurrency of 64 give 256 slots. With the write added, one set of 4 matchers is about 99% busy at 1,000 players a second: it takes about 1,000 a second and has nothing to spare.

The hidden tests ask for that spare. On Saturday night 2,000 players a second arrive for 30 seconds, 1,000 a second more than one set of matchers takes: about 30,000 players piled up in the queue. When a Region goes dark, whatever ran there is gone, and its players have to be matched somewhere else.

## The reference design

Route 53 DNS, with a TTL of 30 s, sends players to two Regions, us-east and eu-west. Each Region is a full stack: an AWS ALB, Go App Servers with 2 instances, a RabbitMQ queue, Celery matchers with 4 instances, a concurrency of 64 and a service time of 200 ms, and a DynamoDB table. Each Region matches its own players, so each sees about 500 a second. This costs about $2,698 per month. DynamoDB is most of it, about $829 per Region, billed per write; the matchers cost $280 per Region, the App Servers $140, RabbitMQ $70, the ALBs $25 and Route 53 $11.

"A weekday evening" passes with a p95 of about 550 ms and no errors, with the matchers about 50% busy.

"Saturday night" doubles the traffic from 15 seconds in, for 30 seconds. Each Region now sees about 1,000 players a second, which its matchers can still take: the p95 is about 573 ms, and nothing fails.

"A Region goes dark" cuts us-east off at 30 seconds, for the rest of a 120-second run. It allows 8% errors, 92% availability and a p95 of 10 s. For about 2 seconds, until DNS health checks notice, us-east's players are still sent there and fail: about 0.80% errors in all. Then eu-west takes every player. Its matchers run at about 100%, but they keep up, and the p95 is about 608 ms.

There is no cost challenge for this problem. The cheapest design we found that passes every test cost about 95% of the reference: DynamoDB's writes are most of the bill, and every match has to be written once, whatever the rest of the design looks like.

## Why the first try breaks

A common first try is one Region, us-east, with the same stack: an AWS ALB, 2 App Servers, RabbitMQ, 4 Celery matchers and DynamoDB. It costs about $2,173 per month and passes "A weekday evening" with a p95 of about 684 ms. But its matchers are 99% busy: there is no headroom.

On Saturday night the queue grows by about 1,000 players a second more than the matchers can take. Waits climb from 5 s to 20 s through the spike, and the p95 is about 29.5 s. The queue kept every player, but it cannot make them wait less: a queue absorbs a burst only if the workers have the headroom to drain it. When us-east goes dark there is nowhere else to go, and about 75% of requests fail.

The point is this: give the matchers headroom, and put a queue and its workers in each Region. Matchmaking is regional anyway, since players want a game near them, so two Regions halve the load each one sees and turn the loss of a Region into a DNS failover.

## What the simulator leaves out

Match quality is not simulated: real matchmakers search by skill range and may wait longer for a better match. Every player costs a fixed 200 ms of matching. Party sizes, the latency between a player and a far Region, and sticky game servers are not simulated either.

## Things to try

- Lower eu-west's matchers to 2 instances and run "A Region goes dark". Can the surviving Region still keep up?
- In the one-Region design, raise the matchers to 8 instances and run "Saturday night". Compare its p95 and cost with the two-Region reference.
- Raise Route 53's TTL and run "A Region goes dark" again. Watch whether the errors before the failover change.
