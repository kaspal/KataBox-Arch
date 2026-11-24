# Nightly Export from PostgreSQL

## The product

You run an online shop. Its orders live in one PostgreSQL database, and the shop's app reads and updates them all day and all night. Every night the data team copies the orders out of that database into their analytics files: an export job, started by a scheduler, reads the orders table page by page. People still shop at night, so the export runs while the app is busy, and the app must stay fast while it does.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 2,000 requests/s (app requests and export pages together) |
| p95 latency | 300 ms or less |
| Availability | 99.9% or more |
| Errors | 1% or less |
| Budget | $1,700 per month |

The p95 counts every request: for the app, the time to its answer; for an export page, the time from the moment it is queued until the page has been read from the database.

## The traffic

- **Open an order** (60%, 1,200 a second). The request may pass a load balancer, reaches your App Servers, and they read one order from the database.
- **Update an order** (10%, 200 a second). The same path, but the App Server writes the order, so it must go to the database's primary.
- **Export a page of orders** (30%, 600 pages a second). A Scheduler card publishes one message per page to a Queue. Export Workers consume the messages and read each page from the database. A page is large, about 576 KB, so it takes the database much longer than reading one order: about 10 times as long. Writing the analytics files is not part of this problem.

A real export runs for hours each night. The simulator can't run hours, so each test simulates one to one and a half minutes of the export at its working rate.

## The tests

One test is visible. You can run it as often as you like:

- **A night with the export running**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **The export carries 3× the data** and **A replica fails during the export**. Your design has to pass all three.
