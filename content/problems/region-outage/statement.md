# Survive a Region Outage

## The product

You run the account service of an app used around the world. People open their account page all day, and now and then they change something: a name, an address, a setting. Every other part of the app asks this service who the user is, so when it is down, the whole app is down. A machine failing is routine. This time the question is bigger: what happens when a whole cloud region goes dark, or stays up but can no longer talk to the rest of the world?

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 6,000 requests/s |
| Reads | 90% of requests |
| p95 latency | 250 ms or less |
| Availability | 99.9% or more |
| Budget | $1,900 per month |

## The traffic

Customers reach the service through an optional edge, such as DNS and a Load Balancer, then an App Server, an optional Cache, and a database. There are 1 million accounts, and their popularity follows a Zipf 1.0 curve: some accounts are read and changed far more often than others.

- **Read an account** (90%). The App Server reads the account from the database, or from a Cache if you add one.
- **Update an account** (10%). The App Server writes the account to the database's primary.

You can place cards in Regions, and Availability Zones inside them. A call between two Regions takes 70 ms each way. A read goes to a database member in the caller's own Region when there is one; a write always goes to the primary, wherever it is.

No outside providers are involved.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The primary's region goes dark** and **The primary's region is cut off for 20 s**. Your design has to pass all three.
