## 1

Almost every request is a search, and the same few searches come back again and again. A few bookings race for the same rooms at the same moment. These are two separate worries: where the popular searches land, and who wins the last room.

## 2

A search engine splits its data into shards, and each search goes to the shard that holds it. Adding shards spreads many different searches, but it cannot split one popular search: all of it still lands on one shard. A cache in front of the search engine answers the popular searches before they get there. For the bookings, read-then-write lets two travellers take the same room; a write that fails when the room-night is already taken lets only one of them win.

## 3

Put an AWS ALB in front of 4 Go App Servers. Add a Redis Cache (4 GB, TTL 300 s) in front of an Elasticsearch card with 4 shards. On the PostgreSQL card, turn on "Conditional writes (check and set in one step)", so the booking is one statement that fails if the room-night is taken.
