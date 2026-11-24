# Partner File Import

## The product

Your company takes in data from hundreds of partners. Each partner's server uploads files to your Upload API: price lists, orders, stock counts. The API stores the file in object storage, adds a message for it to a queue and answers at once. Workers take the messages, read the file back from object storage, parse it and load its rows into PostgreSQL. Partners also check on their imports: a status page reads PostgreSQL. A file that is never loaded is a partner's orders that never ship, and nobody notices until the partner calls.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 50 requests/s |
| p95 latency | 30 s or less, from upload until the rows are in PostgreSQL |
| Availability | 99.9% or more |
| Errors | 0.1% or less |
| Files lost | none |
| Files dead-lettered | none |
| Budget | $4,200 per month |

## The traffic

- **Upload a file** (78%). The partner server calls your App Servers, which write the file (256 KB) to object storage and publish a message to a queue. A Worker consumes it, reads the file from object storage and writes its rows to PostgreSQL. Its p95 is measured until that write is done. Parsing a normal file takes the Worker's default service time of 100 ms.
- **Upload a giant file** (2%). The same path, but the file is 25 times the work at every step after the queue: parsing, reading it back and writing its rows.
- **Check an import's status** (20%). The partner server calls your App Servers, which read the import's row from PostgreSQL.

When a Worker's step fails, the queue tries the message again after 1, 2, 4, 8, 16 and then 32 seconds, up to its "Maximum retries" (3 by default: 7 seconds of waiting in all). A message that fails every retry goes to a Dead-Letter Queue if one is wired to the queue; otherwise it is lost. A file in a Dead-Letter Queue is kept, not loaded: someone has to replay it by hand. The judge counts both: no file may be lost, and none may end up in the Dead-Letter Queue. Aim for retries that outlast a failure on their own, so the Dead-Letter Queue stays empty.

A PostgreSQL card wired with Replicate to a second one keeps a copy. If the primary dies, the replica keeps serving reads, and it is promoted to take writes after the primary's "Failover time" (30 s by default). A lone PostgreSQL that dies takes all reads and writes with it.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A partner sends a month of giant files** and **The database's primary dies**. Your design has to pass all three.
