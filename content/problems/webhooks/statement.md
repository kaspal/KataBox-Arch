# Webhook Delivery

## The product

You run a platform that tells your customers when something happens: an order is paid, a file is ready, a user signs up. Each customer gives you an HTTPS endpoint, and you send every event to it as a webhook. Your Events API receives the event, puts it in a queue and answers at once; Senders deliver it behind it. You do not control the customers' servers. Some are slow, some go down, and your biggest customer, Acme, gets almost a third of all events. Whatever happens on their side, an event must never quietly disappear.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 2,000 events/s |
| p95 completion latency | 5 s or less |
| Availability | 99.9% or more |
| Budget | $10,000 per month |
| Lost messages | None |

## The traffic

Every event takes the same path. The Events API, an App Server, publishes it to a queue and is done. A Worker takes it from the queue and calls the customer's endpoint. Latency is measured until the endpoint accepts the event.

- **Deliver an event** (70%). Goes to the endpoints of customers in general. They answer in about 200 ms at the median, with a slow tail that reaches 2 s.
- **Deliver an event to Acme** (30%). Acme's endpoint answers in about 100 ms at the median.

A Worker keeps its slot for the whole call, so a slow endpoint holds a slot for as long as it takes to answer. A call to an endpoint that is down fails only after a 1 s connect timeout.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **Acme's endpoint is down for a minute** and **Customers' endpoints slow down**. Your design has to pass all three.
