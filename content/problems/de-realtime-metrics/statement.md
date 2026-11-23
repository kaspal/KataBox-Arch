# Real-Time Product Metrics

## The product

Your product team wants a live dashboard: sign-ups, purchases and screen views, counted every 30 seconds, so a bad release shows up in minutes, not in tomorrow's report. Your web app and your mobile app send an event each time a user does something. The events go through a Stream to a Stream Processor, which counts them in 30-second windows by the time each event happened (its event time), not the time it arrived. The counts go to a Time-Series Database, and the dashboard reads them from there.

The web app sends its events within a second or two. The mobile app is different: a phone in a tunnel, on a plane or with the app in the background keeps its events and sends them when it is back online, sometimes minutes later. Those events still belong to the window in which they happened.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 5,000 requests/s |
| p95 time to a counted event | 45 s or less |
| Availability | 99.9% or more |
| Errors | 0.1% or less |
| Late events | 1% or less of the events counted in windows |
| Lost events | none |
| Budget | $2,200 per month |

## The traffic

- **An event from the web app** (97%). The request may pass a load balancer and reaches your App Servers, which publish it to a Stream. The Stream Processor consumes it and adds it to its window. Web events arrive about 0.3 s after they happen (3 s at p99).
- **An event from the mobile app** (2%). The same path. Mobile events arrive about 5 s after they happen at the median, but the tail is long: 120 s at p99.
- **Open the live metrics dashboard** (1%). The request may pass a load balancer and reaches your App Servers, which read the counts from the Time-Series Database.

There are about 20,000 metrics (one per event name and screen).

## How the windows work

- **Window.** The Stream Processor's Window is Tumbling, with a Window length of 30 s: each event counts in the 30-second window its event time falls in.
- **When a result leaves.** A window closes at its end plus the Watermark delay, and then its counts go out. The time to a counted event is measured for each event from the moment it is sent to the moment its window's result leaves the processor. With a 30 s window, that is up to about 35 s even for an event that arrives at once, so the 45 s target leaves about 10 s for everything else.
- **Late events.** An event that reaches the processor after its window has closed updates that window if it is within the Allowed lateness; the update goes out at once. Past the Allowed lateness, it is a late event: counted as late and left out of the counts. It is not lost, but your dashboard is wrong.
- **Throughput.** Each slot of the processor (Instances × Concurrency per instance) handles about 800 events a second here, including reading them from the Stream. A Stream hands each partition to one slot at a time, so the Stream's Partitions cap how many slots can work.

The simulator doesn't send the processor's writes to the Time-Series Database (a few rows per metric per window); wire the Stream Processor to it anyway, as your real pipeline would.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **Phones come back online: 20 times the mobile events** and **Launch day: 3 times the events for 40 s**. Your design has to pass all three.
