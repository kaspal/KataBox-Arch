# Editorial

## Where the load goes

The table holds 365 days in 36.5 GB, so one day is 0.1 GB. Everything depends on how much of the table a search has to read to find that day.

If the table isn't partitioned, the warehouse can't tell where a day starts, so every search reads all 36.5 GB. That takes 0.3 s of overhead + 36.5 s of scanning + 292 files of 128 MB × 20 ms ÷ 8 ≈ 37.5 s, and BigQuery bills 36.5 GB, about $0.22, for each search. Two searches arrive every second, so about 2 × 37.5 = 75 queries would be running at once, far more than any slot count you would give it.

Partitioned by day, a search reads only its day: 0.3 s + 0.1 s + one file ≈ 0.42 s, and 0.1 GB, about $0.0006. Two searches a second then keep about one slot busy.

The year-long queries are different: they need the whole year however the table is laid out, so each one still takes about 37.5 s and costs about $0.22. On a normal day there are about 0.02 a second, less than one slot busy on average. The hidden tests change that. The audit runs 15 times as many, 0.3 a second, which keeps about 11 slots busy on average and more in bursts. The incident multiplies all the traffic by 10 for 30 seconds: about 8 slots for the searches plus about 7.5 for the year-long queries.

## The reference design

The engineers' Client (Timeout 60,000 ms) calls 2 Go App Servers, which query Google BigQuery with 1 instance and Concurrency per instance 32. BigQuery reads an Apache Iceberg table over a Query edge, with Partitioning By day, Compaction Daily (128 MB files) and Stored data 0.04 TB.

It costs about $19,124 a month: $18,983 for BigQuery, $140 for the App Servers and $1 for the table. Of the BigQuery bill, about $3,200 is the 24-hour searches and about $15,800 the year-long queries. The normal run drew 8 of them in 5 minutes, about 1 query in 75, and they are most of the bill. Partitioning can't help a query that really needs the whole year.

"A normal working day" has a p95 of about 417 ms and no errors. "An audit runs fifteen times the usual year-long queries" has a p95 of about 37.4 s, which is the year-long queries themselves, and no errors: 32 slots hold the audit's queries and leave room for the searches. "An incident: everyone searches the last day at once" has a p95 of about 417 ms and no errors.

The judge prices only the normal day. A month at the audit run's traffic would bill about $173,000, and a month at the incident run's traffic about $63,000: an audit that ran all month would blow the budget, and the tests don't check that. Treat year-long queries as the expensive thing they are.

Compaction matters too. Without it (Compaction None), the table keeps the 8 MB files streaming ingest writes. A year-long query then opens 4,672 files instead of 292, takes about 48 s, and the audit fails its p95 of 45 s (48.3 s). The searches barely notice (451 ms).

Partitioning By hour gives the same results here: a 24-hour search reads one day's worth of data either way.

On BigQuery on demand, slots cost nothing here (it bills bytes), so the reference has room to spare. With the warehouse's default Concurrency per instance of 8, the design passes the normal day but fails the audit with 11.4% errors (p95 58.9 s) and the incident with a p95 of 28.9 s: long scans fill the 8 slots, and the searches wait behind them. Concurrency 16 passes everything (the incident's p95 is about 671 ms); 32 leaves headroom.

This problem has no cost challenge. A search for a cheaper design finds one at about 2% of the reference's cost ($321 a month: the same shape and partitioning, with one App Server and a single ClickHouse instance instead of BigQuery), and Snowflake in the same place costs about $1,526. Both bill the hours a warehouse runs, not the bytes it scans, and they scan at the default 1 GB/s, the smallest size: a faster scan would cost more. But they pass only because of a gap the simulator still has (see the caveat below): one small instance runs 32 year-long scans side by side, each at full speed. A proof that rests on a gap is not a lesson, so there is no challenge.

## Why the first try breaks

A common first try is the same design with the table left at Partitioning None. It fails every test, the visible one included.

On "A normal working day" every search reads the whole year and takes about 37.5 s. The 32 slots fill at once, queries queue, and after 60 s the Client gives up: 90.1% of the queries fail, with a p95 of about 60.1 s. The warehouse is 97% busy and the App Servers, waiting on it, 87%. Every query that got a slot (about half of them) was billed for the whole year it read: BigQuery alone comes to about $562,000 a month, 16 times the budget. The audit (90.8% errors) and the incident (92.0% errors) fail the same way.

More slots wouldn't fix it: 75 queries at once at 37.5 s each still miss the 2 s target, and the bill grows with every query that runs. The fix is to read less.

The point is this: a warehouse bills, and waits, for what it reads, not for what you asked. Partition the table by the column your queries filter on, here the timestamp, so a search for one day reads one day. Then size the warehouse for the queries that still read everything.

## What the simulator leaves out

A real search for the last 24 hours usually touches two day partitions (yesterday and today), so it reads up to twice as much; here it reads one day. Real BigQuery on demand runs queries on a shared pool of slots with its own limits; here Concurrency per instance is a prop you set, and it costs nothing. A warehouse billed by time pays for its scan rate per query (a faster scan is a bigger size), but not for how many queries run at once: Concurrency per instance is free, and every query scans at the full rate however many run beside it. That flatters a small Snowflake or ClickHouse warehouse: a real X-Small shares its few cores among its queries, and the year-long queries would take much longer. Columnar tables read only the columns a query names; here a query's size is fixed. BigQuery's result cache is not used: the log table changes all the time, which voids cached results, so no query here repeats. Writing the logs into the table isn't simulated. The monthly bill assumes the peak lasts all month; real traffic falls at night.

## Things to try

- Set Partitioning back to None and run the normal day. Watch the warehouse's slots and the BigQuery cost.
- Set Compaction to None and run the audit. Compare the year-long queries' time with its p95 limit.
- Set Concurrency per instance to 8 and run the incident.
- Swap BigQuery for Snowflake and compare the costs, then read "What the simulator leaves out" again.
