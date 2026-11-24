# Editorial

## Where the load goes

Every event, live or replayed, holds a Flink slot for about 15 ms: about 5 ms of processing, then the write to PostgreSQL and the hops around it. So one slot does about 67 events a second, and a Stream Processor card with 2 instances of 4 slots does about 540. That is the only stage that matters: the App Servers, Kafka and PostgreSQL are each less than 10% busy in every test.

The normal hour asks for 600 events a second (300 live, 300 replayed). The hidden tests change that in two ways:

- The evening peak with the replay sped up sends 750 live and 1,800 replayed events a second: 2,550 in all.
- Losing a Stream Processor card takes away about 540 events a second of processing, for the rest of the test.

The question is who pays for it. A live event is measured by completion, so every second it waits is a second of staleness on the dashboards. A replayed event is measured by response: once its Stream has it, it can wait as long as it likes. A Stream is first in, first out, and the replayed events carry no mark that would let "Live before bulk (priority)" put the live ones first: in a Kafka topic they are just older offsets. So the only way to keep the live events out of the replay's queue is to give the replay a queue, and a processor, of its own.

## The reference design

An AWS ALB sends the events to 2 Go App Servers, which publish them to an Apache Kafka Stream with 16 Partitions and a Retention of 720 hours (30 days). Two Apache Flink Stream Processors, each with 2 instances, a Concurrency per instance of 4 and 200 Records per second per slot, consume it: that is the live consumer group, and it only ever reads new events. They write to PostgreSQL.

The replay is a second pipeline. The Airflow Scheduler, which stands for the replay reading the month back, publishes to its own Kafka Stream with 8 Partitions, and a third Flink card with 2 instances runs the fixed code on it and writes to the same PostgreSQL. In real Kafka this is a second consumer group on the same topic, started at the offsets of 30 days ago; the second Stream card is how the simulator gives it its own offsets and its own partition limit.

It costs about $1,171 a month: $420 for the three Flink cards, $260 for PostgreSQL, $306 for the two Kafka Streams, $140 for the App Servers, $25 for the ALB and $20 for Airflow. In the normal hour the two live Flink cards are about 28% busy each, the replay card about 54%, and the p95 is about 37 ms.

"The evening peak, with the replay sped up to 6 times its pace" sends 750 live events a second to the live consumer group, which is about 68% busy, and 1,800 replayed events a second to the replay card, which can do about 540. The replay card is 100% busy and its Stream fills by about 1,260 events a second, to about 75,000 by the end of the test. None of that reaches the live events: the p95 stays at about 37 ms, with no errors and nothing lost. Speeding up the replay only made the replay's own queue longer.

"A Stream Processor is lost during the replay" kills the first Stream Processor card at 20 s, for good. In the reference that is one of the two live cards. The other one carries the 300 live events a second alone, about 56% busy, and the p95 stays at about 37 ms. The few events the dead card was holding come back after the Stream's Session timeout (45 s) and go to the survivor; they are too few to move the p95. Had the lost card been the replay card, the replay would stop and wait in its Stream, and the live events would not notice. Either way the loss stays on its own side.

Other designs that pass, and some that don't:

- One consumer group for everything, sized for the worst case: two Flink cards with 5 instances each and 40 Partitions pass all three tests at about $1,298, 11% more than the reference. They pass only because the test stops at 6 times the pace: a replay that reads as fast as it can has no such limit, and a shared group never has enough.
- The same with 4 instances per card ($1,158) passes the normal hour and the lost processor, but not the sped-up replay (p95 about 8.6 s).
- The separate replay pipeline, but with one live Flink card of 4 instances instead of two of 2, costs the same as the reference and fails the lost processor with a p95 of about 66 s: the lost card was the only live one.
- A replay card with 1 instance still passes (about $1,101). The replay just takes twice as long; the tests don't judge how long it takes.

There is no cost challenge for this problem: the cheapest design the search found that passes every test costs about $1,031, 88% of the reference.

## Why the first try breaks

The common first try rewinds the live job: one Kafka Stream with 16 Partitions (Retention 720 hours), two Flink cards with 2 instances each, and the Scheduler publishing the replay into the same Stream, so one consumer group does both jobs. It costs about $878, and it passes the normal hour: 600 events a second on about 1,080 a second of processing, each card about 54% busy, with a p95 of about 37 ms.

With the evening peak and the replay sped up, 2,550 events a second arrive and about 1,080 are processed. Both cards are 100% busy, and every new live event joins the back of a pile that grows by about 1,470 events a second. The p95 of each second passes 5 s about 8 s into the test, and the test's p95 is about 49.3 s.

When a Flink card is lost, the survivor gets all 600 events a second and can do about 540. The pile grows more slowly, by about 60 a second, but it never stops: the p95 of each second passes 5 s about 84 s into the test, and the test's p95 is about 7.8 s. In both tests nothing fails and nothing is lost; every event is still in the Stream. The dashboards are just stale.

Turning on "Live before bulk (priority)" changes nothing here: no event is marked as bulk, because a replayed event is the same event read again. That is the difference from the backfill problem (DE 12), whose historical batches came in through the API and could be marked.

The point is this: a replay shares nothing it doesn't have to. Run it as its own consumer group on its own processor, so its speed and its failures stay its own, and keep a spare on the live side, because a live pipeline that loses a processor must still carry the live peak.

## What the simulator leaves out

Kafka consumer groups are not modelled: a Stream card hands each event to one of its consumers. The reference's second Stream card stands for the same topic read by a second consumer group, with its own offsets; in a real cluster it would cost no extra brokers. Retention is not simulated either: Retention 720 hours is there to show the 30 days the replay needs. The replay is a fixed arrival rate from the Scheduler card. A real replay reads as fast as its consumer can and never piles up; here the events it can't process wait in its Stream, which holds up to 100,000 (past that, publishes are refused). How long the replay takes is not judged, and neither is writing the fixed rows to a new version of the table and switching the dashboards to it; both pipelines write to one PostgreSQL card, which has plenty of room. The simulator would let more slots per instance process more, which a CPU-bound job can't; that is why the brief fixes Concurrency per instance and Records per second per slot. A Flink card that dies replays its records since the last checkpoint, and the simulator counts those duplicates (about 3,000 in the reference's lost-processor test), but this problem doesn't judge them (see DE 11, Clickstream Sessionization). Consumer group rebalances, which pause a real shared group when a member leaves, are not simulated. The whole replay takes days; each test sees one or two minutes of it.

## Things to try

- In the reference, wire the Scheduler to the live Stream instead of the replay Stream, and run the sped-up replay. Watch the live events wait behind the month.
- Replace the two live Flink cards with one of 4 instances and run the lost processor. Same capacity, same cost, and nothing left when it dies.
- Give the replay card 1 instance and watch its Stream during the sped-up replay. The replay's processor decides how long the replay takes, and nothing else does.
