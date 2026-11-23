# Editorial

## Where the load goes

An agent task is a chain of calls, and the Worker waits for each one while holding its slot. A model call with the median sizes takes about 2.2 s: 600 ms before the first token, about 0.3 s to read 1,500 tokens at 5,000 a second, and about 1.25 s to write 100 tokens at 80 a second. A long one (3,000 tokens in, 250 out) takes about 4.3 s. The tools are quick by comparison: tens to hundreds of milliseconds.

So a question (two model calls) takes about 5 s, and a refund (four) about 9.5 s. At 2 tasks a second, with 80% questions, a Worker card keeps about 2 × (0.8 × 5 + 0.2 × 9.5) ≈ 12 slots busy at once. A Celery Worker has 8 slots per instance.

The model limits are not the problem here. 2.4 model calls per task is about 290 calls a minute against 4,000, and about 460,000 tokens a minute at the median sizes (the long calls push the real average about a quarter higher) against 2,000,000.

What breaks a design is time. When tracking takes 40 s a call, a task that waits for it holds its slot 40 s longer: about 2 × 46 ≈ 92 slots busy at once. And in returns week the mix changes: 1.6 questions and 2 refunds a second need about 1.6 × 5 + 2 × 9.5 ≈ 27 slots.

## The reference design

A Web client sends tasks to a Go App Server, which publishes them to an Amazon SQS Queue. A Celery Worker with 5 Instances (40 slots) and the Call timeout set to 10,000 ms consumes them and calls a Claude API Model Inference card with the Tokens per minute limit at 2,000,000. It costs about $94,674 a month, and the tokens are almost all of it: about $94,176 on the Claude API, $350 for the Worker, $140 for the App Server and $7 for the Queue. On a normal day the p95 to completion is about 12.3 s, with no errors, and the Worker is about 35% busy.

The Call timeout is one number for every call the Worker makes, so it has to fit the slowest call the task can't do without: a long model call, about 4.3 s. 10 s leaves room for that and still caps a slow tool.

"The carrier's tracking API slows to 40 s a call" adds 40 s to every tracking call from 20 seconds in, for 60 seconds, in a run of 2 minutes. The Worker stops waiting for tracking after 10 s, and because tracking is optional, the agent goes on and answers from the order record. The tracking call it gave up on runs on, unwanted, but no longer holds a slot. A task now takes about 10 s longer, so about 31 of the 40 slots are busy during the slowdown. The p95 is about 19.4 s, with no errors, and the Worker is about 62% busy over the run.

"Returns week: five times the refunds" multiplies refunds by 5 for the 2 minutes of the run. About 27 slots are busy at the median sizes, and the 40 slots have room for the long tasks too: the Worker is about 72% busy, the p95 is about 16.3 s, and there are no errors. The token bill for that mix is about twice a normal day's, and the account's 2,000,000 tokens a minute still cover it.

There is no cost challenge for this problem. The tokens are more than 99% of the bill and the traffic sets them, so the cheapest design found that passes every test costs the same as the reference.

## Why the first try breaks

A common first try is the same chain with a Celery Worker of 3 Instances (24 slots) and no Call timeout. It costs about $94,534 and passes "A normal day" with the same p95 of about 12.3 s; its Worker is about 59% busy.

When tracking slows down, it fails with a p95 of more than a minute. Every task waits the full 40 s for tracking, so each one holds its slot about 46 s instead of 6. The 24 slots fill within seconds, new tasks wait in the Queue behind them, and the wait grows for as long as the slowdown lasts. From about 64 seconds in, the tasks finishing each second have a p95 between 36 and 66 s. The Worker is about 91% busy. Nothing fails: tracking still answers, just too late to be useful, which is why the agent should stop asking.

Returns week fails too, with a p95 of about 31.9 s: about 27 slots' worth of work arrives for 24 slots, so the Queue grows and the Worker is about 97% busy.

Fixing one half is not enough:

- The first try with the Call timeout at 10,000 ms passes the slowdown (a p95 of about 22.3 s) and still fails returns week with about 31.9 s: a timeout bounds a task, but doesn't add slots.
- 5 Instances without a Call timeout pass returns week and fail the slowdown with a p95 of about 61.3 s. 8 Instances still fail it, at about 51.3 s: about 92 slots' worth of waiting is more than any sensible pool.

The point is this: every call an agent makes needs a time limit, and the steps the task can do without should be allowed to fail. Then one slow tool costs each task at most its timeout, instead of holding every task and every slot. And size the workers for the longest chains you expect, with headroom, because each step of an agent holds the slot.

## What the simulator leaves out

The quality of the answers is not simulated: an answer written without the tracking result counts the same as one with it. A real agent decides its steps as it goes, so the number of model and tool calls varies from task to task; here each kind of task has a fixed chain. The Call timeout is one value for all of a Worker's calls; real code usually sets a timeout per tool and a time or step budget for the whole task. Tool calls made in parallel, retries of a single tool call, and a cap on the agent's iterations are not simulated. The tools have no price here, and the tracking calls the Worker gave up on cost nothing. The slowdown lasts 60 s in a 2-minute run; a real one can last much longer, and then the backlog of the first try never drains. The token prices are the registry's teaching figures.

## Things to try

- Remove the Call timeout from the reference and run the slowdown. Watch the Worker's slots fill.
- Set the Worker to 3 Instances in the reference and run returns week. How many slots does the new mix need?
- Lower the Tokens per minute limit to 400,000 and run returns week. Which limit runs out first now?
