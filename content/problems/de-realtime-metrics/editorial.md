# Editorial

## Where the load goes

5,000 events a second reach the App Servers, which only publish them to the Stream: that part is easy. The work is in the Stream Processor, and it has two separate limits.

The first is time. A 30-second tumbling window with a 5 s watermark delays every result by up to 35 s, about 33 s at p95, whatever you do. That leaves about 10 s of the 45 s target for queueing. An event that reaches the processor after its window closed is late unless the Allowed lateness still covers it. Web events (0.3 s median, 3 s p99) are almost never late. Mobile events (5 s median, 120 s p99) often are: an event at the end of its window has only the 5 s watermark before the window closes.

The second is throughput. A slot handles about 800 events a second here, so 5,000 events a second need about 6.3 busy slots, and the launch's 15,000 need about 19. The Stream's Partitions cap how many slots can work.

The hidden tests take each limit in turn. "Phones come back online" multiplies the mobile events by 20: 2,000 mobile events a second, about 29% of all events instead of 2%. "Launch day" triples all the traffic for 40 seconds, from 20 s into a run of 90 s.

## The reference design

An AWS ALB sends the traffic to 4 Go App Servers. They publish events to an Apache Kafka Stream with 32 Partitions and read the dashboard from a TimescaleDB Time-Series Database. An Apache Flink Stream Processor of 8 instances with a Concurrency per instance of 4 (32 slots, one per partition) consumes the Stream: Window Tumbling, Window length 30 s, Watermark delay 5 s, Allowed lateness 180 s. It is wired to the Time-Series Database with a Query edge. This costs about $1,278 per month: $560 for the Stream Processor, $280 for the App Servers, $260 for TimescaleDB, $153 for Kafka and $25 for the ALB.

On a normal day the processor is about 19% busy and the time to a counted event is about 33.2 s at p95, with no errors. Fewer than 0.03% of the windowed events are late.

The Allowed lateness is sized from the delay distribution: 180 s is more than the mobile p99 of 120 s, even for an event at the very end of its window (5 s + 180 s). A late mobile event updates its closed window at once instead of being dropped. So when the phones come back online, the late rate stays under 0.03%, and the p95 stays at about 33.2 s.

The watermark stays at 5 s. A longer watermark would also catch more stragglers, but it delays every result, not just the late ones: a 120 s watermark would put every count on the dashboard more than two minutes behind.

On launch day, 15,000 events a second meet 32 slots that can take about 25,600 a second: about 59% busy at the peak. There are no errors, and the p95 stays at about 33.2 s.

For the cost challenge, a proof design comes in at 67% of the reference's cost, under the 70% target. It keeps the 32 Partitions and the same window settings, with 2 App Servers and 4 Flink instances (16 slots): at the launch peak they run over capacity, but the backlog they build in 40 s is small enough to clear without errors and within the 45 s target.

## Why the first try breaks

A common first try uses the same edge and App Servers, a Kafka Stream with 12 Partitions, and a Flink Stream Processor of 3 instances with a Concurrency per instance of 4 (12 slots), with the same 30 s tumbling window and 5 s watermark but the default Allowed lateness of 0. It costs about $928 and passes "A normal day": the processor is about 52% busy, the p95 is about 33.2 s and between 0.1% and 0.2% of the events are late, all of them mobile.

When the phones come back online, mobile events become about 29% of the traffic. Every mobile event that arrives after its window's end plus 5 s is dropped from the counts: between 2% and 4% of all windowed events are late, over the 1% limit, and the test fails as an SLO breach. The processor has capacity to spare; the windows simply close too early for these events.

On launch day, 15,000 events a second meet 12 slots that can take about 9,600. From about 18 s into the spike the processor's backlog overflows, and about 36% of the events each second are refused until the spike ends: 13.6% errors over the run. More instances alone would not help: the 12 Partitions would leave any extra slots idle.

The point is this: a window needs two numbers, not one. The watermark sets how fresh the results are; the allowed lateness sets how many late events still count. Size the lateness from how late your slowest clients really are, and size slots and partitions together for the peak.

## What the simulator leaves out

The processor's writes to the Time-Series Database are not sent: an event counts as done when its window's result leaves the processor (the real writes are a few rows per metric per window). A late update is not billed or timed as an extra write. Keeping windows open for the allowed lateness costs memory in a real processor; here the window state does not grow with it. Event time comes from a delay drawn for each event, not from clocks on the devices, so wrong phone clocks are not simulated. The dashboard's reads are simple lookups, and the counts themselves are not checked: only the late events are counted.

## Things to try

- Set the Allowed lateness to 60 s and run "Phones come back online". Compare the late rate with 0 s and 180 s.
- Raise the Watermark delay to 30 s instead, with an Allowed lateness of 0. Watch both the late rate and the p95.
- Keep 8 Flink instances but set the Stream to 12 Partitions, and run "Launch day".
