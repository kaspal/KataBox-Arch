# Clickstream Sessionization

## The product

You run the analytics behind a large shopping site. Every click on the site (a page view, a search, an "add to cart") is sent to your backend, and the product team wants each user's current session in a table they can read: which pages, in what order, how long so far. A Go collector receives the clicks and writes them to a Kafka stream. A stream processor reads the stream, keeps every open session in its state, keyed by user, and writes each updated session to a NoSQL table.

The analysts read that table every few seconds, so a session should show a click a few seconds after it happens. They have one more request: when the processor crashes and restarts, it must not write a flood of repeated session updates into the table. They can live with a few seconds' worth (at most 6,000 replayed results per crash, 15 s of clicks at peak), but not minutes of them.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 400 clicks/s |
| p95 time from click to the sessions table | 10 s or less |
| Availability | 99.9% or more |
| Errors | 0.1% or less |
| Lost clicks | none |
| Replayed (duplicate) results per crash | 6,000 or less |
| Budget | $2,500 per month |

## The traffic

- **Record a click** (100%). The click may pass a load balancer, reaches your App Servers, goes into a Stream, is processed by a Stream Processor and is written to a data store. It is measured until its result reaches the data store. There are 1,000,000 users, and some are much more active than others (a Zipf popularity of 0.8).

How to model the sessions:

- A real session ends after 30 minutes without a click. The simulator runs for a minute or two, not hours, and has no session windows, so time is scaled down: model a session as a sliding window (**Window** `sliding`) with a **Window length** of 60 s and a **Slide** of 1 s. The processor then updates each user's session every second. Keep the **Watermark delay** at 5 s.
- Each open session keeps about **128 KB** of state (its click path, timestamps and attributes): set **State per key** to 128 KB.
- A Stream Processor slot handles 20,000 simple records a second (the **Records per second per slot** default). A sliding window costs window ÷ slide times that per click: here 60 times.
- When the processor's state is larger than its **Memory** times its **Instances**, it spills to disk, and the simulator makes every record 4 times slower.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The stream processor crashes** and **Four times as many users**. Your design has to pass all three.
