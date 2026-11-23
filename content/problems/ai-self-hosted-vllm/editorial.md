# Editorial

## Where the load goes

The App Servers and the load balancer do little: they pass each message on and wait for the answer. The work is on the GPUs, and a GPU's limit is the tokens it can write each second, not the number of slots it has.

Before the first token, a message needs 200 ms plus its prompt read at 5,000 tokens a second: 0.2 + 1,000 / 5,000 = 0.4 s for a typical prompt, 0.6 s for a long one. That is the time to first token (TTFT) when nothing waits. The limit is 1 s, so there are only a few hundred milliseconds left for queueing.

Then the answer is written. One answer alone goes at 80 tokens a second; with B of the 64 slots busy, each goes at 80 × (1 − 0.5 × B ÷ 64). The batching trade-off, in numbers for one GPU:

| Busy slots | Speed of each answer | A 250-token answer takes | The GPU writes in total |
| --- | --- | --- | --- |
| 16 | 70 tokens/s | 3.6 s | 1,120 tokens/s |
| 32 | 60 tokens/s | 4.2 s | 1,920 tokens/s |
| 48 | 50 tokens/s | 5.0 s | 2,400 tokens/s |
| 64 | 40 tokens/s | 6.3 s | 2,560 tokens/s |

A fuller batch writes more tokens in total, and each answer is slower. The last quarter of the batch (48 to 64) adds only 7% more tokens a second, and makes every answer 25% slower. And at 64 there is nothing left: the next message waits for a slot, and its wait counts in its TTFT.

The peak is 20 messages a second of about 250 tokens each: 5,000 tokens a second, more in practice because long answers pull the average up (the 250 is a typical answer, not the average). Double traffic is 10,000 tokens a second or more. One GPU writes at most 2,560, so double traffic needs four GPUs at 100% before anything else goes wrong, and a GPU at 100% queues.

The GPU test stops the first vLLM card from 20 seconds in until the end of a run of 2 minutes. Its GPUs are gone, and the other cards must carry the normal peak.

## The reference design

A Web client with a Timeout of 30,000 ms, 2 Retries, a Retry backoff of 200 ms and the Backoff jitter set to Full (random wait) sends messages to an AWS ALB, which spreads them over 4 Go App Servers. They call three vLLM on GPU cards in turn, each with 2 Instances (2 GPUs) and Batch slots per GPU at 64: 6 GPUs. It costs about $4,709 per month: $4,404 for the 6 GPUs, $280 for the App Servers and $25 for the ALB.

On a normal day the GPUs are about 22% busy (about 14 of 64 slots each), the TTFT p95 is about 0.6 s, the whole answer's p95 is about 8.8 s, and there are no errors.

"Twice the traffic for 2 minutes" doubles the traffic from 20 seconds in, for 2 minutes, in a run of 150 seconds. The six GPUs share 40 messages a second, about 6.7 each, and are about 50% busy over the run: more during the double traffic, with room left in every batch. Nothing waits for a slot: the TTFT p95 stays at about 0.6 s, and the whole answer's p95 rises to about 10.7 s because each answer is written a little slower in a fuller batch. There are no errors.

"A GPU card is lost" leaves 4 GPUs for the normal peak, 5 messages a second each: the same load per GPU as the first try on a normal day. For a second or two, until two health checks in a row find the card down, the App Servers keep sending a third of the messages to it; those calls fail at once, and the client's retries, after a random wait of up to 200 ms, reach a working card. About 0.12% of messages fail, the TTFT p95 is about 0.6 s and the whole answer's p95 about 9.7 s.

Three cards of 2 GPUs is a size that covers both tests: six GPUs for double traffic, and four left when a card goes. Two cards of 3 GPUs pass too, at the same cost (about 0.41% errors when a card is lost, and a p95 of about 10.9 s), since 3 GPUs still carry the normal peak.

There is no cost challenge for this problem. The search for a cheaper design that passes every test found none: one GPU fewer, or a smaller batch, fails the double traffic.

## Why the first try breaks

A common first try counts slots. Two vLLM cards of 2 GPUs at 64 Batch slots per GPU hold 256 answers at once; at the peak, 20 messages a second of about 4 seconds each are only about 80 answers in flight, and at double traffic about 160. It looks like plenty. With the same client, ALB and 4 App Servers, it costs about $3,241 per month, and it passes "A normal day": the GPUs are about 36% busy, the TTFT p95 is about 0.6 s and the whole answer's p95 about 9.5 s, a little slower than the reference because the batches are fuller.

At double traffic, each of the 4 GPUs gets 10 messages a second. More answers in the batch slow every answer down, so each one holds its slot longer, so even more answers are in the batch: the batches fill up to 64. Then a full GPU writes 2,560 tokens a second and is asked for more, and the extra messages wait for a slot. The GPUs are about 87% busy over the run, which means full for the whole double traffic. From about 24 seconds into it, the whole answer's p95 is over 15 s and climbs to about 29 s; the TTFT p95 for the test is about 21 s, against a limit of 1 s. There are no errors (the client waits up to 30 s), but it fails both latency limits.

When a card is lost, the 2 GPUs left get 10 messages a second each: the same overload, from the first seconds after the loss. The surviving card is about 87% busy over the run, the TTFT p95 is about 18.5 s and the whole answer's p95 about 28 s.

Other first tries break in instructive ways:

- Halving the batch makes it worse, not better: three cards of 2 GPUs at 32 Batch slots per GPU (the reference's GPUs) fail double traffic with about 45.9% errors. A GPU with 32 slots writes at most 32 × 40 = 1,280 tokens a second.
- Four cards of 1 GPU (the first try's 4 GPUs, spread out) survive the loss of a card, because only one GPU goes, and still fail double traffic.
- Uneven cards break: cards of 2, 2 and 1 GPUs fail both hidden tests. The App Servers take turns between cards, not between GPUs, so the 1-GPU card gets as many messages as a 2-GPU card and fills up first. Keep the cards the same size.
- One card of 6 GPUs has the reference's capacity and fails the GPU test with about 86.7% errors: when it goes, there is nothing left.

The point is this: size self-hosted GPUs by the tokens they write, not by the slots they hold. A full batch gives the most tokens a second and the slowest answers, and leaves no room, so a little more traffic becomes a queue, and the queue lands on the time to first token. Run the GPUs half to two-thirds busy at your worst moment, and split them into equal cards so the loss of one leaves enough.

## What the simulator leaves out

The quality of the answers is not simulated: a fuller batch, or another model, gives the same answers as far as the simulator is concerned. GPU memory is not simulated: the limit of 64 answers at once on one GPU comes from the statement, not from the simulator. The simulator would let you set Batch slots per GPU to 128, and then the first try's 4 GPUs pass every test; a real GPU would run out of memory for the conversations (the KV cache) and pause or refuse some of them. The slowdown in a batch is a straight line, a modelling choice; real speeds depend on the model, the GPU, the prompt lengths and vLLM's settings. Starting a GPU server and loading the model are not simulated, and a Model Inference card can't be autoscaled here. The GPU price is the registry's teaching figure (a GPU node at $734 a month, an hourly price × 730), not a quote.

## Things to try

- Set the first try's Batch slots per GPU to 32 and run "A normal day". Look at how busy the GPUs are.
- Remove the third card from the reference and give the other two 3 Instances each. Run "A GPU card is lost" and compare its numbers with the reference's.
- Give the reference's third card 1 Instance and run the double traffic. Which card fills up first, and why?
