# Audit Log

## The product

You run the audit log for a business software company. Every product it sells reports what its users do: who signed in, who changed a permission, who exported a report. Each of those is an audit event, and your service records it. Compliance teams and customers then search the trail to answer "who did what, and when". An event may reach the search results a little late, and nobody minds. What nobody accepts is an event that was never recorded, so none may be lost.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 2,000 rps |
| p95 latency | 100 ms or less |
| Errors | 1% or less |
| Availability | 99.9% or more |
| Budget | $2,200 per month |
| Lost messages | None |

## The traffic

- **Record an audit event** (97%). A product sends the event to an App Server, and the event ends up in a Search Index. The App Server may hand it to a Stream, and then a Worker takes it from the Stream and writes it to the index. With no Stream wired, the App Server writes the index itself, inline, and the answer waits for that write. Latency is measured until the event is acknowledged.
- **Search the audit trail** (3%). An App Server queries the Search Index and returns the matching events.

A load balancer in front of the App Servers is optional. There are no outside services in this problem.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A batch job logs everything at once** and **The search cluster is down for 30 s**. Your design has to pass all three.
