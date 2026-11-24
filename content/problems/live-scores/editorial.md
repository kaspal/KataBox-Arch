# Editorial

## Where the load goes

Most of the load is the open connections, not the page loads. 5,000,000 fans each receive an update every 50 seconds on average: 5,000,000 ÷ 50 = 100,000 messages a second. Each open connection also holds about 20 KB of memory on its gateway, so 5,000,000 fans need about 100 GB in all. A gateway instance with 32 GB could fit 1.6 million connections by memory, but its own limit is 1,000,000.

The hidden tests push both sides. A late goal multiplies the updates by ten for 10 seconds: 1,000,000 messages a second, along with ten times the page loads. A dead gateway drops every fan it held at once, and those fans all want to reconnect. The rest of the gateways need room for them, and they need to accept them at a pace they can manage.

The match pages are the easy part: 2,000 rps over 2,000 matches, with a few big matches taking most of the views, so a short-lived cache absorbs most reads.

## The reference design

The Client, with Reconnect jitter set to 20 s, connects through an AWS ALB to 4 Socket.IO WebSocket Gateway cards. Each card runs 2 instances with Max connections 1,000,000, Memory 32 GB, Messages per second 200,000 and New connections per second 20,000. That gives 8,000,000 connection slots for 5,000,000 fans (60% full) and 1,600,000 messages a second. The match pages go to 4 Go App Servers, a Redis Cache with a TTL of 2 s and PostgreSQL. This costs $1,131 per month: PostgreSQL $260, the App Servers $280, $140 for each gateway card, the ALB $25 and Redis $6.

"A busy match day" passes with no errors, and every fan stays connected.

"A goal in the last minute" sends ten times the messages per fan (1,000,000 a second) and ten times the page loads, from 20 seconds in, for 10 seconds. It passes with no errors: the messages stay under the 1,600,000 a second the gateways can push, and the App Servers go from about 7% busy to about 70%.

"A gateway dies at half-time" kills the first gateway card for good at 20 seconds, in a run of 120 seconds, and lowers the connected target to 70%. It passes with no errors. The 1,250,000 fans on that card come back over about 20 seconds onto the 6 remaining instances, which have room for 6,000,000 and can accept 120,000 new connections a second. The connected share dips to about 75% at the kill and recovers.

The jitter is what makes this work. Without it, the same reference fails the test: about 98% of connection attempts are refused for over 30 seconds. Headroom alone doesn't help when 1,250,000 clients reconnect in the same 100 ms. With a random wait of 0 to 20 seconds, they arrive at about 62,000 a second, under the survivors' 120,000.

There is no cost challenge here: nothing cheaper passed every test. The gateways are cheap; the database is the largest line.

## Why the first try breaks

A common first try has no jitter on the Client and 3 Socket.IO gateway cards of 2 instances, with the same settings except Messages per second 100,000, and 2 Go App Servers. That is 6,000,000 slots for 5,000,000 fans (20% headroom) and 600,000 messages a second. It costs $851 per month and passes "A busy match day".

"A goal in the last minute" fails with 10.7% errors and a p95 of 6.2 s. A burst of 1,000,000 messages a second meets 600,000 a second of capacity, and the backlog it builds takes about 10 seconds to drain after the burst ends.

"A gateway dies at half-time" fails badly: 99.7% of connection attempts are refused. 2,000,000 fans lose their gateway at once and all retry in the same instant, every second, so the survivors refuse nearly all of them. And even with perfect pacing, the 4,000,000 slots left can't hold 5,000,000 fans, so 1,000,000 would never get back.

The point is this: size connection gateways for N−1, so that losing one still fits everyone; keep message headroom for bursts; and make clients reconnect with jitter (and backoff), so that a failure becomes a steady stream of reconnects instead of a storm.

## What the simulator leaves out

There are no per-match rooms: every fan receives updates at the platform's average rate, not only for the match they follow. Sticky sessions and connection state are not modelled, nor are mobile networks dropping connections on their own. The CPU cost of TLS handshakes, the ALB's own connection limits and bandwidth are not simulated either.

## Things to try

- Set the Client's Reconnect jitter to 0 on the reference and run "A gateway dies at half-time". How long do the refused attempts last?
- Lower each gateway card's Messages per second to 100,000 on the reference and run "A goal in the last minute". Does the backlog come back?
- Remove the fourth gateway card from the reference, keep the jitter, and run the gateway test. Is jitter enough when the survivors have no room?
