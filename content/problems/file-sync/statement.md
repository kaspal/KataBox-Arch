# Cloud File Sync

## The product

You run a file sync service. People keep folders on their phones and laptops, and when a file changes, the device cuts it into chunks and sends you the chunks that changed. You keep every chunk, keep a record of which chunks make up which file, and tell the user's other devices that something changed so they can fetch it. Many chunks reach you more than once: the same file sits on several devices, and a shared folder holds the same files for many people. A change a device has sent must never be lost.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 2,000 requests/s |
| p95 completion latency | 5 s or less |
| Availability | 99.9% or more |
| Budget | $4,000 per month |
| Lost messages | None |

## The traffic

Devices reach the Sync API, an App Server, usually through a Load Balancer.

- **Upload a changed chunk** (85%). Each chunk is 128 KB. If your design has a Cache in front, the Sync API first looks the chunk up there: a hit means the service already has that chunk, and the upload is done. Otherwise the chunk is stored in object storage, the file's record is written to the database, and a change notice is published to a queue, where a Worker picks it up and tells the user's other devices. Latency is measured until the other devices have been told, not until the Sync API answers.
- **List a folder** (15%). The Sync API reads the folder's records from the database.

There are 2,000,000 distinct chunks, and how often each one is uploaded follows a Zipf 1.2 curve: a fairly small set of chunks makes up most of the uploads. No outside providers are involved. Keep in mind that object storage and queues are billed per request.

## The tests

One test is visible. You can run it as often as you like:

- **Normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A new laptop syncs everything** and **The object store slows down**. Your design has to pass all three.
