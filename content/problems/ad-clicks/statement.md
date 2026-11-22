# Ad Click Aggregator

## The product

You run click counting for an ad network. Each time someone clicks an ad, the ad server that showed it reports the click to you, and you add it to that ad's totals. Advertisers pay by the click, so the totals are money: they may lag behind for a little while, but a click must never be lost. Advertisers and the billing team read an ad's totals now and then. A few ads get most of the clicks, and the reports do not always arrive at an even pace.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 10,000 requests/s |
| p95 completion latency | 30 s or less |
| Availability | 99.9% or more |
| Budget | $8,000 per month |
| Lost messages | None |

## The traffic

Ad servers reach a Click collector, an App Server, usually through a Load Balancer.

- **Record a click** (99%). The collector publishes the click to a stream and answers at once. A Worker, the aggregator, takes the click from the stream and adds one to the ad's counter, which your design keeps either in a Cache or in a database. Latency is measured until the click is counted, not until the collector answers.
- **Read an ad's totals** (1%). The collector reads the ad's counter from wherever your design keeps the counters.

There are 100,000 ads, and their clicks follow a Zipf 1.1 curve: a small set of ads gets most of them. No outside providers are involved.

The 30 s limit is generous on purpose. Nobody needs a click counted within milliseconds; what matters is that every click is counted, even when many arrive at once.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **One ad goes viral** and **Late events arrive all at once**. Your design has to pass all three.
