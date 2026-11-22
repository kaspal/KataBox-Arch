# Payments Ledger

## The product

You run the money-moving core of a payments app. Every transfer takes an amount from one account and gives it to another, and the app has to remember each one for good. The ledger is the record of every transfer, in order, and it is the source of truth. Account balances are what people look at, but they are only a summary of the ledger, kept up to date from it. When the app tells a customer "done", that transfer must never vanish, even if a database machine dies the next second.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 1,000 transfers/s |
| p95 completion latency | 3 s or less |
| Availability | 99.9% or more |
| Budget | $3,800 per month |
| Lost messages | None |

## The traffic

There is one request type, and every request takes the same path.

- **Make a transfer** (100%). The payment apps reach the Ledger API, an App Server, usually through a Load Balancer. The App Server commits the transfer to a SQL database, the ledger, and the customer gets the answer as soon as the commit is done. The ledger then publishes the committed change to a stream (change data capture). A Worker takes it from the stream and updates the account balances in a NoSQL database. There are 5 million accounts.

Latency is measured to completion: until the balance has been updated, not just until the customer gets the answer. A transfer that is committed but never reaches the balances counts as lost.

No outside providers are involved.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A retry storm** and **The ledger's primary fails mid-transfer**. Your design has to pass all three.
