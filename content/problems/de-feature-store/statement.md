# ML Feature Store

## The product

Your company runs machine learning models that make predictions about users while they use the app: what to recommend, whether a session looks risky. Each prediction needs the user's features, a row of about 2 KB of numbers about that user (purchases this week, time since the last visit, and so on). You run the feature store: the service that hands those features out.

It has three customers. The model servers read one user's features for every prediction, and they need the answer fast. The app sends events (a click, a purchase), and a stream job turns each event into updated features for that user, which the model servers should see within moments. And the data scientists' training jobs read large batches of historical features from the Data Warehouse, where every feature also lands. The models themselves run elsewhere and are not part of this problem.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 1,000 requests/s |
| p95 latency (answers to reads, and completion of updates) | 50 ms or less |
| Availability | 99.9% or more |
| Errors | 0.1% or less |
| Lost updates | 0 |
| Budget | $4,000 per month |

## The traffic

- **Read a user's features for a prediction** (80%). The request may pass a load balancer and reaches your App Servers. They may look in a Cache first (cache-aside: on a miss, the answer from the store fills the Cache), and then read the user's row from a NoSQL Store or from the Data Warehouse. There are 1,000,000 users, and the most active ones make most of the reads (a Zipf popularity of 1.2).
- **Update a user's features from an event** (19.8%). An App Server publishes the event to a Stream. A Stream Processor consumes it and writes the user's new features to the online store: a NoSQL Store, or a Cache written directly. This latency is measured to completion: from the event until the online store has the new features. It is how fresh the features are. Events follow the same popularity as the reads.
- **Read training features** (0.2%). A training job asks the offline feature service, a Microservice, which runs a query in the Data Warehouse that scans about 0.5 GB of history.

The Data Warehouse (Snowflake by default) spends about 300 ms on every query before it reads any data, plus the scan: about 0.5 s more for a training read, at 1 GB/s. Each instance runs 8 queries at a time ("Concurrency per instance"); the rest wait in its queue. Snowflake bills an instance only while it is running, plus its auto-suspend time.

A Stream hands out one record per partition at a time, and a Stream Processor's slot holds a record until its write to the online store is done. How many records are in flight at once is the smaller of the Stream's partitions and the Stream Processor's slots (instances × concurrency per instance).

The daily job that loads features into the warehouse, and the first copy of every user's features into the online store, happen before the tests start. The tests don't include them.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A sale: three times the traffic for 30 s** and **The stream processor slows down for 30 s: features must still arrive within 1 s**. Your design has to pass all three.
