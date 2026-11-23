# Editorial

## Where the load goes

5 documents a second of about 625 tokens each is about 3,100 tokens a second, or about 190,000 tokens a minute: a third of the 600,000 your account allows. The request limit is far away: 300 requests a minute against 3,000. So tokens are the limit that matters. In documents, 600,000 tokens a minute is 600,000 / 625 = 960 documents a minute, about 16 a second.

The provider doesn't count per second. It gives you a minute's allowance at a time and refills it continuously, so a short burst can spend tokens faster than the limit until the allowance runs out. Then every call past the refill rate gets a 429.

Each embedding takes the provider about 0.6 s for a typical document: 500 ms before the answer starts and about 0.1 s to read 500 tokens at 5,000 tokens a second. Whoever makes the call keeps a slot for all of it.

## The reference design

A Web client sends uploads to 2 Go App Servers. Each publishes the document to an Amazon SQS Queue with Maximum retries set to 10, and answers. A Celery Worker with 2 instances and a concurrency of 12 consumes the queue: it calls the OpenAI API card (Model type Embeddings, 3,000 requests and 600,000 tokens a minute) and writes the vector to a pgvector Vector Database. It costs about $20,971 per month: $20,415 of it is embedding tokens, $260 the Vector Database, $140 each for the App Servers and the Workers, and $16 the queue. A normal day has a p95 completion of about 939 ms, and the Workers are about 16% busy.

Why 24 Worker slots? A document holds a slot for about 0.74 s (the Worker's own 100 ms, the embedding call and the vector write), so 24 slots handle about 32 documents a second: twice the steady limit of 16. That is enough to spend the minute's allowance during a burst instead of leaving it unused. More slots than that buy nothing: the provider's limit is the ceiling.

"A customer imports their archive (×10 for 45 s)" sends 50 documents a second from second 20 to second 65, in a run of 120 s. The queue takes them all at once, and the Workers embed about 32 a second, spending the allowance faster than it refills. The rest wait in the queue, about 800 documents at the end of the import, and are done some 30 s later. Once the allowance runs out, calls start to get 429s. Each such message goes back to the queue and is tried again after 1 s, 2 s, 4 s and so on, so a 429 costs time, not a document. No upload fails, nothing is lost, and the p95 completion is about 32.5 s. This test allows a p95 of 60 s: a big import may take a little longer to become searchable, but every document must get there.

"The embeddings API slows down" adds 5 s to every embedding from second 20, for 30 seconds. Each document now holds a Worker slot for about 5.7 s, so the 24 slots manage about 4.2 documents a second while 5 arrive. A small backlog of a few dozen documents builds up and clears within seconds once the provider recovers. The uploads never notice: the App Servers only publish. The p95 completion is about 9.5 s, with no errors.

There is no cost challenge for this problem: tokens are 97% of the bill, and every document must be embedded once at the same price. The cheapest proof the search found costs $20,901, 100% of the reference.

## Why the first try breaks

A common first try has no queue: the 2 Go App Servers call the embeddings API and the Vector Database themselves while the upload waits. It costs about $20,815 and passes "A normal day" with a p95 of about 802 ms.

The import breaks it. 50 documents a second is about 31,000 tokens a second, against a refill of 10,000 a second, so the minute's allowance of 600,000 tokens is gone after about 600,000 / 21,000 ≈ 28 s. From second 49 to the end of the import, only about 16 of every 50 documents a second get through, and the rest are refused with a 429. Nothing holds them for later, so each refusal is a failed upload: about 15.9% of the run's documents fail.

The slowdown breaks it too. A typical embedding now takes about 5.6 s, longer than the Web client's 5 s timeout, so every upload in those 30 seconds times out: about 24.4% of the run's documents fail. The App Servers are not short of slots (they are about 3% busy on a normal day); the upload is simply waiting on someone else's model.

The point is this: a rate-limited, slow provider belongs behind a queue. A 429 means "not now", so give the document a durable place to wait and retry it with backoff. Size the Workers to the provider's limits, not to the traffic: enough slots to spend the allowance, and no more.

## What the simulator leaves out

Only latency, throughput, limits, cost and failures are measured. Whether the embeddings are any good, and whether search finds the right documents, is not simulated at all. Splitting long documents into chunks, parsing files and building the vector index are not simulated either. The token price is the registry's teaching price for the OpenAI API flavour, a chat-model list price still marked as needing a source; check your provider's own price for embeddings. The batch API's delivery window is not simulated as time; instead, a batch call for work that someone is waiting on (as here) fails. The provider's limits are one bucket per minute for requests and one for tokens; real providers may count differently and send a Retry-After header, which is not simulated. The queue's backoff doubles from 1 s up to 32 s and is not configurable. A document set aside in a dead-letter queue is not counted as a failure by the simulator, but it is still a document missing from search until someone sends it again. In this simulator a hosted model has no slot limit, so a slowdown costs your Workers' slots, not the provider's capacity.

## Things to try

- Set the queue's Maximum retries to 3 and run the import. Documents that are refused four times in a row are lost.
- Cut the Workers to a concurrency of 6 (12 slots) and run the import. The Workers can't spend the allowance during the burst, and the backlog takes over a minute to clear.
- Raise the Workers to 8 instances with a concurrency of 16 and run the import. The p95 doesn't move: the provider's limit is the ceiling, and the extra slots only collect 429s.
