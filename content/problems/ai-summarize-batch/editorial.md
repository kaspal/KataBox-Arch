# Editorial

## Where the load goes

Each document is about 600 tokens in and 100 tokens out, so about 700 tokens a call. Tonight's backlog is 450 documents: about 315,000 tokens. Your account allows 400,000 tokens a minute in real time, and the limit starts with one full minute's allowance, so tonight's backlog fits in it with room to spare. The request limit (4,000 a minute) is never the problem here; with 700 tokens a call, the token limit runs out first.

Three times the backlog is 1,350 documents, about 945,000 tokens. The first 400,000 go through at once; the rest trickle in at 400,000 a minute, about 9.5 documents a second, so the last documents wait about 80 seconds more. The deadline is 45 seconds. No number of Workers changes that: the limit is on the provider's side.

The other cap is your own Workers. A call takes about 2 seconds (600 ms to the first token, 120 ms to read the prompt, about 1.25 s to write 100 tokens at 80 tokens a second). 2 Celery instances with a concurrency of 64 hold 128 calls at once, about 64 documents a second, more than the 45 a second of the biggest test.

## The reference design

An Airflow Scheduler publishes each document to an Amazon SQS Queue (Maximum retries 10), wired to a Dead-Letter Queue. 2 Celery Workers with a concurrency of 64 consume it and call a Claude API Model Inference card with "API mode" set to "Batch (cheaper, slower)". In batch mode the token price is half, and the per-minute limits don't apply. This costs about $18,760 per month: about $18,587 of tokens, $140 for the Workers, $20 for the Scheduler and $13 for the Queue. Tonight's backlog has a p95 of about 3.1 s from queued to summarised, with no errors and nothing lost.

"Three times the backlog" queues 45 documents a second for 30 seconds. With no per-minute limit, the Workers take them as fast as they arrive (about 90 calls at once, of 128), and the p95 stays at about 3.5 s.

"The model provider slows down" adds 3 seconds to every model call for the first 90 seconds, with the normal backlog. A call now takes about 5 seconds, so 15 documents a second need about 75 calls at once, still under 128. The p95 is about 6.4 s, with no errors.

For the cost challenge there is no target: the search found nothing cheaper than the reference by a useful margin (its best proof cost 100% of it). Almost the whole bill is tokens, and batch mode has already halved them.

## Why the first try breaks

A common first try is the same design with the model in real time, the card's default. It costs about $37,347 per month, twice the tokens, and passes "Tonight's backlog" with the same p95 of about 3.1 s, because 315,000 tokens fit in the first minute's allowance. It also passes "The model provider slows down" (p95 about 6.4 s): a slower provider doesn't spend more tokens.

"Three times the backlog" breaks it. Once the first minute's tokens run out, calls come back with an HTTP 429. A refused call fails the message, and the Queue retries it after a backoff, while new documents keep arriving. The documents wait behind the token limit, and the p95 climbs past a minute, against a deadline of 45 seconds. The Workers are about 7% busy on a normal night: they are not the bottleneck. With the same design and the token limit lifted, the test passes, which shows that the limit is the cause. Fewer retries don't save it either: with the Queue's "Maximum retries" at 3, about 371 documents give up and go to the Dead-Letter Queue, and the test fails on them, because none of them was summarised.

The point is this: when nobody waits for an answer, the real-time API sells you speed you don't need, at full price, with a per-minute limit on top. Work with a deadline goes through the batch API: half the token price and no per-minute limit, and then the only cap is the Worker capacity you size yourself.

## What the simulator leaves out

A real batch API doesn't answer in seconds: you submit a batch, and the results come back later, within a window the provider sets. In this simulator a batch call takes as long as a real-time one: its delivery window is not simulated as time (the simulator only refuses batch calls for work someone is waiting on, and this backlog is marked as work that can wait), which is why the scaled deadline of 45 seconds works at all. In a real job, check that the provider's batch window fits your deadline. Batch APIs also have limits of their own (how many requests a batch or a queue may hold), which are not simulated. The simulator does not stop you from raising the "Tokens per minute limit" on the card; a real provider does. The answer quality of the summaries is not measured at all. Prompt caching discounts, Retry-After headers and the cost of storing the summaries are not simulated, and the monthly cost is the run's rate kept up for a month, not the bill of one nightly job.

## Things to try

- Switch the model to "Real time" and run "Three times the backlog". Watch when the 429s start, and compare with 400,000 tokens a minute.
- In real time, double the Workers' concurrency and run it again. More slots only send more calls into the same limit.
- In batch mode, drop the Workers to 1 instance with a concurrency of 32 and run "The model provider slows down". Compare the calls in flight with documents a second × seconds per call.
