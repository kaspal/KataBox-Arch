# Editorial

## Where the load goes

At 3,000 requests a second, 60% are scores and 40% are events: 1,800 scores and 1,200 events a second. Each score waits about 20 ms on the model, so 1,800 × 0.02 s keeps about 36 scores in flight at the model, and the same number of API slots busy waiting for it. That is light work for two GPUs.

The hidden tests change those numbers. On Black Friday the traffic triples for 30 seconds: 5,400 scores a second, about 108 in flight, and 3,600 events a second, 150 a second for each of 24 partitions. When the model slows down by 80 ms, a score takes about 100 ms: on its own that is past the 60 ms p95, and 1,800 × 0.1 s keeps about 180 payments waiting at once. The question is how long the API is willing to wait.

## The reference design

Partner servers (B2B) send every request to an AWS ALB, which spreads it over 6 Go API instances with a Call timeout of 40 ms. For a score, the API reads the card's features from a Redis cache with 16 GB of Memory, then calls an NVIDIA Triton Model Inference card with 2 instances and Model type set to Classifier. For an event, the API publishes to an Apache Kafka stream with 24 Partitions; Apache Flink, with 2 instances, consumes it and writes the updated features to the same Redis. This costs about $2,494 per month, and the GPUs are most of it: Triton is $1,468, the API $420, Redis $288, Kafka $153, Flink $140 and the ALB $25. On a normal day the p95 is about 28 ms, with no errors, and the GPUs are about 13% busy.

"The model server slows down" adds 80 ms to every model call from 20 seconds in, for 30 seconds. With the Call timeout, the API stops waiting after 40 ms: a late score is dropped and the rules decide, so the payment still succeeds. The p95 rises to about 53 ms, with no errors.

"Black Friday" triples the traffic from 15 seconds in, for 30 seconds. There is room for it everywhere: the p95 stays at about 28 ms, with no errors.

There is no cost challenge for this problem. The two GPUs are more than half of the bill, and the cheapest design found that passes every test still came to just over 80% of the reference, over the challenge's cap.

## Why the first try breaks

A common first try is exactly the same design without a Call timeout. It costs the same $2,494, passes "A normal day" with a p95 of about 28 ms, and passes "Black Friday" too.

When the model slows down, it breaks: about 8.7% errors and a p95 of about 1.6 s. With no limit, every payment waits the extra 80 ms for its score. Each waiting payment holds an API slot, the slots fill up, new requests queue behind them, and many of them fail. The model step was optional all along, but nothing told the API to give up on it.

The point is this: an in-line ML call needs a latency budget. A timeout with a rules fallback keeps checkout fast when the model is slow. The two designs are otherwise identical; one setting is the difference.

## What the simulator leaves out

The model's accuracy is not simulated, and neither is the quality of the rules fallback: a payment decided by rules counts the same as one the model scored. Feature freshness, how late Flink's updates reach Redis, is not measured. Payment amounts and chargebacks are not simulated either.

## Things to try

- Remove the Call timeout and run "The model server slows down" again. Watch the API's slots fill while the model is slow.
- Set the Call timeout to 15 ms, below the model's usual 20 ms, and run "A normal day". How many payments are now decided by rules?
- Drop Triton to 1 instance and run "Black Friday". Watch how busy the GPU gets and whether the p95 moves.
