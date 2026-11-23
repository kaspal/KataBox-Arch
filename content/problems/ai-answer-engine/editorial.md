# Editorial

## Where the load goes

Start with the chat model, because it is the only part with a hard limit and almost all of the bill. A question averages about 1,850 tokens (the medians are 1,500 in and 250 out; the long prompts and answers pull the average up). At 200 questions a second, sending every question to the model is 12,000 requests a minute, 75% of the 16,000 limit, and about 22 million tokens a minute, 74% of the 30 million limit. It fits on a normal day, with a quarter to spare. Each question costs about $0.0088 (1,574 tokens in at $3 per million, 273 out at $15 per million), so 200 a second is about $4.65 million a month.

The per-minute buckets start full with one minute's allowance, so a short burst goes through; what breaks them is more demand than the limit for longer than a minute. The hidden tests do exactly that. "A story breaks: everyone asks the same few questions" triples the traffic from 10 s to the end of a 180 s test and makes the questions much hotter (a Zipf popularity of 1.2 instead of 0.95). "The evening peak lasts four minutes" doubles the traffic for 240 s, with the usual popularity.

So the real question is how many questions the model must answer. Popular questions arrive word for word, and an answer may be reused for 10 minutes. A cache keyed on the exact question answers every question that was already asked in the last 10 minutes, and it does it before the search, the embedding and the model. The hotter the questions, the more it answers. A cache that matches by meaning can only answer the near-repeats, which the traffic fixes at a quarter, however hot the topic gets.

## The reference design

The Web client (Timeout 20,000 ms) sends questions to an AWS ALB and 8 Go App Server instances with a Concurrency per instance of 512. An App Server holds its slot for the whole answer, about 5 s, so 200 questions a second keep about 1,000 slots busy at most: it needs many slots, not much CPU. The App Server reads a Redis Cache first, Mode "Exact keys", its default 2 GB, with a TTL of 600 s, the most the freshness rule allows. In 10 minutes at most 120,000 questions arrive (360,000 in the storm), and 2 GB holds about 520,000 answers of 4 KB, so the memory is not the limit: the TTL is.

On a miss the App Server queries an Elasticsearch Search Index with its defaults (2 shards, 1 replica), embeds the question on NVIDIA Triton (2 instances, Model type "Embeddings", Prompt speed 50,000 tokens/s), searches a Qdrant Vector Database with 2 shards of 50 million passages each (Vectors (millions) 100), and calls a Claude API Model Inference card at the account's limits. The simulator gives the embedding step the same 1,500-token prompt as the chat model, so at Triton's default 5,000 tokens/s each embedding would take 300 ms and the embedding tier, not the model's limit, would be the first thing to fill up; 50,000 tokens/s keeps it at about 35 ms.

It costs about $2,674,011 per month: $2,671,152 for the model's tokens, $1,468 for Triton, $560 for the App Servers, $520 for Qdrant, $250 for Elasticsearch, $36 for Redis and $25 for the ALB. The budget of $5 million is about 1.9 times that. On a normal day the cache answers about 42% of the questions, so about 115 a second reach the model: about 6,900 requests and 12.8 million tokens a minute, 43% of each limit. The p95 is about 6,541 ms, set by the questions that go to the model, with no errors. The retrieval tiers are lightly loaded (Search Index 11%, Vector Database 9%, Triton 3%, App Servers 12%).

In the storm, the cache answers about 86% of the questions: fewer than 90 a second reach the model while 600 arrive, about 31% of the request limit. The p95 drops to about 4,671 ms, because more answers come from the cache, and there are no errors. In the long peak, more questions repeat within the 10 minutes, so the cache answers about 44%: about 224 a second reach the model, about 13,400 requests and 24.8 million tokens a minute, 84% of the limits. The p95 stays at about 6,541 ms with no errors.

A CloudFront CDN with a TTL of 600 s in front of the ALB, instead of the Redis Cache, works as well: answers are the same for everyone, so the edge can keep them. It passes every test at about $2,663,267.

The cost challenge found no cheaper design that passes (no challenge, after 60 evaluations): the model's tokens are over 99% of the bill, and the share of questions a cache may answer is set by the traffic and the 10-minute rule.

## Why the first try breaks

A common first try is the same pipeline with no cache: every question goes to the search, the embedding, the vector search and the model. It passes "A normal day" with a p95 of about 7,222 ms, at 75% of the request limit. It costs about $4,648,109 a month, 1.74 times the reference and 93% of the budget: the cost ceiling is nearly spent before anything goes wrong.

When the story breaks, 600 questions a second become 36,000 requests a minute, 2.25 times the limit. The bucket's minute of allowance runs out about 48 s after the storm starts; from about 57 s in, around 55% of the calls get a 429, and the test ends with about 39% errors. The rest of the pipeline is fine: even at three times the traffic, the Search Index and the Vector Database are only about half busy. Only the model's limit breaks.

In the long peak, 400 questions a second are 24,000 requests a minute, 1.5 times the limit. The bucket lasts about two minutes; from about 120 s in, about a third of the calls get a 429, and the test ends with about 17% errors. At that rate its token bill would be about $7.7 million a month.

A cache that matches by meaning doesn't save it. A Redis in Mode "Semantic (near-repeats)" spends 15 ms per lookup on one thread, so 200 lookups a second are 3 s of work a second: it saturates and drops about two thirds of the questions on a normal day. A multi-threaded Dragonfly with 128 GB in semantic mode passes the normal day and the long peak, but it can never answer more than the quarter of questions that are near-repeats, so in the storm about 450 questions a second still reach the model, and it ends with about 59% errors.

The point is this: in front of a hosted model with per-minute limits, the questions you don't send are your capacity. Trending traffic repeats itself word for word, so an exact answer cache in front of the whole pipeline grows with the trend, keeps the model under its limits, and cuts the token bill nearly in half.

## What the simulator leaves out

The simulator models request and token limits, latency, cache hit rates and token cost, not answer quality. It doesn't check whether a cached answer is still right: the 10-minute freshness rule is the statement's, and the card accepts any TTL you type, which is why the statement fixes it (a longer TTL would answer more and be wrong more often about the news). An exact cache's hits come from each question's popularity rank and the TTL; normalising questions (case, spaces) is not modelled. A provider cutting your limits in the middle of an incident is not simulated either: a test can't change a Model Inference card's limits, so the hidden tests raise the demand against a fixed limit instead. The card accepts any rate limit you type, which is why the statement fixes them. The embedding step is charged with the whole prompt's tokens, as said above, and self-hosted Triton is billed per instance, not per token. Latency is counted to the last token; streaming is not simulated. Crawling and updating the index are not simulated, and search and vector search times come from the simulator's formulas, not from benchmarks. Prices are the registry's teaching prices for a Sonnet-class hosted model.

## Things to try

- Switch the Redis Cache's Mode to "Semantic (near-repeats)" and run "A normal day". Watch the cache's utilisation; then try a Dragonfly with 128 GB in semantic mode and work out how many questions reach the model in a storm at a quarter near-repeats.
- Set the TTL back to its default of 300 s. The cost rises to about $2.89 million; compare the long peak's requests per minute with the limit.
- Replace the Redis Cache with a CloudFront CDN (TTL 600 s) in front of the ALB, and compare the cost.
