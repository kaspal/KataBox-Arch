# Editorial

## Where the load goes

A question uses about 500 + 120 = 620 tokens. The account allows 400,000 tokens a minute, about 6,700 a second, so at most about 10.75 questions a second can reach the model. The peak is 12 a second: sending every question to the model is already over the limit on a normal day, and every question sent costs about a fifth of a cent (500 tokens in at $2 per million, 120 out at $10 per million).

So the question is how many of them the model really has to answer. The traffic says it: 60% of the everyday questions mean the same as an earlier one, and 95% of the viral topic's do. But the same meaning is rarely the same text. An exact-key cache only hits on the common, identical wordings (here about a third of the questions); a semantic cache compares meanings and can answer every near-repeat it has seen.

The hidden tests push on both halves. "One question goes viral: three times the traffic" multiplies the viral topic by 41, so it becomes about two thirds of 36 questions a second, written in 50,000 different ways. "The evening peak, and a cache node restarts" runs 1.5 times the traffic for 180 s and kills the cache 20 s in, for 5 s.

## The reference design

The Web client sends questions to a Go App Server card (2 instances). The App Server reads a Redis Cache with Mode "Semantic (near-repeats)" and 2 GB of memory, wired with Replicate to a second Redis Cache with the same settings. On a miss it calls a Claude API Model Inference card with its defaults and puts the answer in the cache. 100,000 wordings at 8 KB each (the answer and its embedding) are about 800 MB, so 2 GB holds them all.

It costs about $27,743 per month: about $27,554 for the model's tokens, $140 for the App Servers and $24 for each Redis. Almost all of it is tokens. On a normal day about 4.6 questions a second reach the model, about 171,000 tokens a minute, 43% of the limit. The p95 is about 3,144 ms, set by the questions that go to the model; a semantic lookup adds about 15 ms (the embedding and the search), far less than any model call. The 0.14% of errors are the longest answers, which run past the client's 5 s timeout.

In the viral test, the near-repeats help most where they are needed: about 5.8 questions a second reach the model, about 215,000 tokens a minute, even though three times as many questions arrive. The p95 drops to about 2,683 ms, because more answers come from the cache, and errors stay at 0.07%.

In the restart test, the killed node fails the requests that reach it in the roughly 2 seconds before the health checks notice; then the replica answers. When the node comes back, it resyncs the replica's copy instead of starting empty. Errors are about 0.75%, under the test's 3%.

The cost challenge found no cheaper design that passes (no challenge, after 60 evaluations): the model's tokens are over 99% of the bill, and the share of questions a cache can answer is set by the traffic, not by the design.

## Why the first try breaks

A common first try is the same App Server and model with one Redis Cache of 2 GB in its default Mode, "Exact keys", with the default TTL of 300 s. It costs about $46,259, because about two thirds of the questions still reach the model, and passes "A normal day" with a p95 of about 3,271 ms and about 283,000 tokens a minute, 71% of the limit.

When the topic goes viral, the exact cache barely helps: 50,000 wordings spread evenly almost never repeat word for word. About 32 questions a second reach the model, close to 1.2 million tokens a minute, three times the limit. The token bucket holds one minute's allowance, so the first half minute still goes through; from about 26 s in, around half of the calls get a 429, and the test ends with about 43% errors.

The restart test doesn't catch it. The cache comes back empty, but every answer from the model is put back in it, and the common wordings are asked again within seconds, so it soon answers its third of the questions again. The requests that reach the killed node in the 2 seconds before the health checks notice fail, and the test ends with about 1.3% errors, under its 3%. It passes only because the test is short: at 1.5 times the traffic the exact cache still sends the model a little more than its limit of tokens a minute, so the bucket slowly drains. In a run of 25 minutes the 429s start about 5 minutes in, and it ends with about 4.2% errors.

A single semantic cache, without the replica, fails the restart test (about 21% errors): it has to see the questions again before it can match them, and seeing 100,000 wordings again at 18 a second takes hours.

The point is this: in front of a hosted model, the cache decides how much of your token limit and your bill you spend. Match questions by meaning, because that is how people repeat them, and keep that cache's copy alive, because without it all the traffic lands on the model at once.

## What the simulator leaves out

The simulator models hit rates, the lookup's latency, token limits and token cost, not answer quality. A real semantic cache can return a wrong answer for a question that only looks similar, and the similarity threshold that trades hits against those mistakes is not a setting here: the hit rate is declared by the traffic, and no design can raise it. The embedding step is a fixed 15 ms and is not billed. Answers in the cache never go out of date when a help article changes. Latency is counted to the last token; streaming is not simulated. The Batch API's delivery window is not simulated as time; instead, a batch call for a question someone is waiting on fails. The card accepts any rate limit you type, which is why the statement fixes them. Prices are the registry's teaching prices for a Sonnet-class hosted model. Redis persistence is not simulated, so a lone Redis always comes back empty. The two kinds of cache refill differently here. An exact-key cache is filled key by key: it starts warm (every wording asked at least once per TTL is in it), and each answer from the model is put back in it, so after a restart the popular wordings return within seconds. A semantic cache's refill is a simpler curve: it matches a near-repeat in proportion to how many of the distinct wordings it has seen since it was emptied, without favouring the popular ones, so it relearns more slowly than a real one would.

## Things to try

- Switch the reference's caches to "Exact keys" and run the viral test. Compare the tokens per minute with the limit of 400,000.
- Remove the replica and run the restart test. Watch when the 429s start, and how long the cache would take to see 100,000 wordings again.
- Send every question straight to the model with no cache, and run "A normal day". Work out why it fails from 620 tokens × 12 questions a second.
