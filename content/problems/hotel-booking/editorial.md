# Editorial

## Where the load goes

At 5,000 rps, 4,900 requests a second are searches and 100 are bookings. The searches spread over 200,000 different queries, but not evenly: with this skew the single most popular search is close to 8% of them, nearly 400 a second. A search engine sends each search to the shard that holds it, so that one search always lands on the same shard, however many shards there are. When holiday bookings open, the traffic triples to 15,000 rps: 14,700 searches a second.

The bookings are few, but 20% of them, 20 a second, are for the same 10 rooms. In the last-room test bookings go up 8 times, to 800 a second, and 160 of them a second race for those 10 rooms: about 16 a second for each room.

So the load splits into two separate problems. The searches are many, repeated and read-only, and the popular ones pile onto one shard. The bookings on the wanted rooms race each other for the same room-night.

## The reference design

An AWS ALB sends travellers to 4 Go App Servers. Searches go cache-aside through a Redis Cache with 4 GB of memory and a TTL of 300 s, in front of Elasticsearch with 4 shards. Bookings go to PostgreSQL with "Conditional writes (check and set in one step)" on. This costs about $1,114 per month: $500 for Elasticsearch, $260 for PostgreSQL, $280 for the App Servers, $49 for Redis and $25 for the ALB. In "A normal week" the p95 is about 27.1 ms, with no errors, and the search engine is about 14% busy.

With conditional writes, the booking is one statement that fails if the room-night is already taken: for example a unique constraint on (room, night), or `UPDATE … SET booked = true WHERE room = ? AND night = ? AND NOT booked`. The database checks and writes in the same step, so when two bookings race for the last room, one wins and the other is told "just booked by someone else".

The cache is what keeps the search engine calm. The popular searches are answered from Redis and never reach Elasticsearch, so no shard gets a hot spot. Tuning showed why it is required: without a cache, even 20 Elasticsearch shards ($2,500 for search alone) still dropped about 1% of searches on a normal week, because the most popular search still went to one shard, and the 4 App Servers filled up waiting on search.

"Holiday bookings open" triples the traffic from 15 seconds in, for 30 seconds. The extra searches are mostly the same popular ones, so the cache absorbs them: the p95 is about 26.1 ms, with no errors.

"Everyone wants the last room" multiplies the bookings by 8 and allows no conflicts at all. Every booking is simulated in this problem, not a sample. The conditional write lets one booking win each room-night: the p95 is about 31.8 ms, with 0 conflicts. The losers get an answer, not an error: the room was just booked by someone else.

The cache is allowed to be a little stale. About 22% of searches come from a cached result older than the last booking, so a search can show a room that has just gone. That is fine: the search is only a hint, and the booking is what decides.

For the cost challenge, a proof design comes in at 67% of the reference's cost, under the 70% target. It keeps the same shape and the conditional write, with fewer App Servers, a much smaller cache and fewer search shards: once the cache takes the popular searches, the search engine has little left to do.

## Why the first try breaks

A common first try is the same design with PostgreSQL's conditional writes left off: the App Server reads the availability, then inserts the booking. It costs the same, about $1,114 per month, and passes "A normal week" and "Holiday bookings open" with the same results.

"Everyone wants the last room" breaks it. 160 bookings a second race for 10 rooms, and many pairs both read "free" before either writes. Both succeed: 1,992 double bookings in one run, each one a traveller who will arrive to find the room taken. The test allows none, so the design fails.

The point is this: two separate habits. Cache the hot searches, because sharding spreads many searches but never splits one popular search. And let the database refuse the second booking, with a constraint or a conditional write, instead of checking first and writing after.

## What the simulator leaves out

Elasticsearch replicas are not simulated: the replicas prop does not change the result. Real availability calendars and multi-night bookings are not simulated either; here a booking is one key. Payment holds and overbooking policies are left out. A lock service, a Redis or etcd lock, would also prevent double bookings, but a constraint in the database is simpler.

## Things to try

- Turn off "Conditional writes (check and set in one step)" in the reference and run "Everyone wants the last room". Count the conflicts.
- Remove the Redis Cache from the reference and run "A normal week". Then raise Elasticsearch to 20 shards and run it again: does the error rate go away?
- Set the cache's TTL to 30 s and run "A normal week". Compare the stale reads and how busy the search engine is with a TTL of 300 s.
