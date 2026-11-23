# Editorial

## Where the load goes

The App Servers and the load balancer do little: they pass each keystroke on and wait. The time goes on the GPUs, and the budget is tight.

On an idle GPU, a completion needs 15 ms plus its prompt read at 50,000 tokens a second, then its tokens at 500 a second. A typical request takes 15 + 800 / 50,000 s = 31 ms to its first token and 10 / 500 s = 20 ms to write: about 51 ms. A long one (2,000 tokens of code and a 30-token completion) takes 55 + 60 = 115 ms. That already uses most of the 150 ms budget, with the GPU doing nothing else.

Now fill the batch. With B of the 64 slots busy, each completion is written at 500 × (1 − 0.5 × B ÷ 64) tokens a second. With a full batch, the long completion's 60 ms of writing becomes 120 ms, and the request takes 175 ms: over the budget before it waits for anything. And a full GPU makes the next request wait for a slot. So a design for this budget needs GPUs that stay well short of full, even at the worst moment.

The peak is 1,500 requests a second, and the busy hour is 6,000. At about 50 ms each, that is about 75 completions in flight at the peak and 300 in the busy hour, more in practice because busier GPUs write slower. Four GPUs have 256 slots.

The other lever is to send fewer requests to the GPUs. An identical prefix gets the same completion, and a few prefixes are typed very often. A Redis Cache of 4 GB holds about 4 million completions of 1 KB; with a TTL of an hour, it keeps the prefixes that come back, and a hit costs a millisecond instead of a GPU slot. In these runs it answers roughly 4 in 10 requests, and in the busy hour it takes its share of four times the traffic.

## The reference design

A Web client with a Timeout of 2,000 ms, 2 Retries, a Retry backoff of 20 ms and the Backoff jitter set to Full (random wait) sends keystrokes to an AWS ALB, which spreads them over 6 Go App Servers. They read a Redis Cache with 4 GB of Memory and a TTL of 3,600 s. On a miss, they call three vLLM on GPU cards in turn, each with 2 Instances (2 GPUs), Batch slots per GPU at 64 and the model's speeds from the brief: 6 GPUs. It costs about $4,921 per month: $4,404 for the 6 GPUs, $420 for the App Servers, $72 for the Redis Cache and $25 for the ALB.

On a normal day the GPUs are about 15% busy, the p95 is about 106 ms and there are no errors.

"A busy hour: four times the typing" quadruples the traffic from 20 seconds in, for one minute, in a run of 90 seconds (a real busy hour lasts an hour; one minute is long enough for any queue to show). At four times the traffic the GPUs are about 71% busy and the App Servers about 83%. Nothing waits for a slot, and the p95 rises to about 127 ms, because each completion is written a little slower in a fuller batch. There are no errors.

"A GPU card is lost" stops the first vLLM card from 20 seconds in until the end of the run. Four GPUs and the cache carry the normal peak. For a moment, until the health checks find the card down, the App Servers keep sending a third of the misses to it; those calls fail at once, and the editor's retry, after a random wait of up to 20 ms, reaches a working card. About 0.04% of requests fail and the p95 is about 109 ms.

Two cards of 3 GPUs pass too, at the same cost (about 0.14% errors when a card is lost, and a p95 of about 111 ms).

The cache is the cheapest part of the design and saves the most. Without it, the same 6 GPUs fail the busy hour with about 23% errors. Eight GPUs without a cache (four cards of 2, about $6,457) still fail it, with about 1.2% errors. Nine GPUs without a cache (three cards of 3) pass every test, but cost about $7,191, over the budget of $7,000. A $72 cache does the work of three or more GPUs at $734 each.

There is no cost challenge for this problem. The search for a cheaper design that passes every test found nothing under 80% of the reference's cost: its best was 99%. One GPU fewer, or a smaller cache, fails the busy hour.

## Why the first try breaks

A common first try sends every keystroke to the model. One vLLM on GPU card of 4 Instances, Batch slots per GPU at 64 and the brief's speeds holds 256 completions at once, more than the 75 in flight at the peak. With the same client, ALB and 6 App Servers, and no cache, it costs about $3,381 per month and passes "A normal day": the GPUs are about 38% busy and the p95 is about 122 ms, slower than the reference because the batches are fuller.

In the busy hour, the 4 GPUs get 6,000 requests a second. Fuller batches write every completion slower, so each holds its slot longer, so the batches fill: the GPUs are 100% busy, and requests wait for a slot. The App Servers, each holding a request while it waits, fill up and refuse the rest. During the busy minute nearly half of the requests fail, about 42.8% over the whole test, and the p95 is about 1,215 ms. The GPUs are the limit, not the App Servers: with 24 App Servers instead of 6, the busy hour still fails, with more errors, because more requests reach the full GPUs and wait there.

When the card is lost, there is nothing left: every request from 20 seconds in fails, about 77.8% of the test.

Other first tries break in instructive ways:

- A cache in front of two cards of 2 GPUs (about $3,453) fails the busy hour with about 15.1% errors. The cache saves GPUs; it doesn't replace headroom.
- Six GPUs as one card have the reference's GPUs and lose them all at once when the card goes.

The point is this: in an interactive budget, a GPU's headroom is latency. A fuller batch is slower for every request in it, and a full one queues, so size the GPUs to stay well short of full at the busiest moment, in equal cards so the loss of one leaves enough. And don't spend a GPU on a request a cache can answer: a repeated prefix answered in a millisecond is both the fastest answer and the cheapest.

## What the simulator leaves out

The quality of the completions is not simulated: another model, a fuller batch or a cached answer give the same completion as far as the simulator is concerned. vLLM's own prefix caching (reusing the GPU's work on a shared prefix inside the model server) and speculative decoding are not simulated; the cache here is a separate Redis Cache of finished completions. Real editors wait for a pause in typing before they ask, and cancel a request when the next keystroke makes it useless; neither is simulated, so every request runs to the end. GPU memory is not simulated: the limit of 64 completions at once on one GPU comes from the brief. The slowdown in a batch is a straight line, a modelling choice. Here is how the simulator fills the cache. It starts warm, as after an hour of traffic: every prefix typed at least once in its TTL is in it, as many as its 4 GB hold. Then every completion from the model is put in the cache (cache-aside), so a prefix that missed and comes back within the TTL is a hit. In tests of 90 seconds this second part adds almost nothing: the prefixes that come back that soon are the popular ones, and they are already in the warm cache. Either way the cache answers about 38% of the requests. Starting a GPU server and loading the model are not simulated, and a Model Inference card can't be autoscaled here. The GPU and cache prices are the registry's teaching figures, not quotes.

## Things to try

- Remove the cache from the reference and run the busy hour. How many GPUs does it take to pass without it, and what do they cost?
- Give the first try's card 6 Instances and run "A GPU card is lost". Then split them into three cards of 2.
- Set the reference's Batch slots per GPU to 32 and run the busy hour. A GPU with fewer slots writes fewer tokens a second in total.
