# Page-View Counter

## The product

You run a counting service for websites. Every time someone opens a page, the site sends you a small report, and you add one to that page's count. Site owners open a stats dashboard to see how their pages are doing. The reports arrive in a steady flood, a few pages are far more popular than the rest, and every view must end up in the count: none may be dropped.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 3,000 requests/s |
| p95 completion latency | 3 s or less |
| Availability | 99.9% or more |
| Budget | $1,700 per month |
| Lost messages | None |

## The traffic

Websites reach a Collector API, an App Server, usually through a Load Balancer.

- **Count a page view** (99%). The Collector publishes the view to a stream and answers at once. A Worker takes the view from the stream and adds it to the page's count in the database. Latency is measured until the view is counted, not until the Collector answers.
- **Open the stats dashboard** (1%). The Collector reads the page's count, from a Cache first if your design has one, otherwise from the database.

There are 1,000,000 pages, and their popularity follows a Zipf 1.0 curve: a handful of pages gets a large share of the views. No outside providers are involved.

If you cache the counts, the dashboard can show a number a few seconds old. That is fine here: nobody minds a dashboard that is a little behind, so stale reads are not judged.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The homepage goes viral** and **Consumers slow down**. Your design has to pass all three.
