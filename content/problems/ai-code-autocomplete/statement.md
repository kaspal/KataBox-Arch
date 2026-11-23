# Code Autocomplete as You Type

## The product

You build the autocomplete of a code editor used by a large company's developers. As someone types, the editor sends the code before the cursor (the prefix) to your backend and shows the rest of the line in grey; Tab accepts it. A suggestion is useful only if it shows up before the next keystroke, so the whole round trip has a budget of about 150 ms. The code must not leave the company's own cloud account, so you run a small open-weights code model yourself, with vLLM, on GPUs you rent by the month. Near a release deadline everyone types at once, and GPU servers do fail.

## What you must meet

| Goal | Target |
| --- | --- |
| Peak traffic | 1,500 completions/s |
| p95 latency | 150 ms or less |
| Availability | 99% or more |
| Errors | 1% or less |
| Budget | $7,000 per month |

## The traffic

- **Suggest the rest of the line** (100%). The request may pass a load balancer, reaches your App Servers, may be answered from a cache (cache-aside), and otherwise goes to a Model Inference card for a completion. A request sends about 800 tokens of code to the model (2,000 at p95) and gets back a short completion of about 10 tokens (30 at p95).

The same prefix comes back again and again: a developer deletes a character and types it again, and the same boilerplate (imports, the first lines of a test, a loop header) is typed all over the company. In an hour there are about 15 million distinct prefixes, and a few of them are typed far more often than the rest (a Zipf popularity of 0.6). The completion for an identical prefix can be reused, and one cached completion takes about 1 KB.

The editor waits at most 2 s for a suggestion. A failed request may be retried by the editor (the Client card's Retries) after a short wait.

## Your GPUs

Use **vLLM on GPU** Model Inference cards for your small code model, and set its speeds on every card: **Time to first token** 15 ms, **Prompt speed** 50,000 tokens a second and **Output speed** 500 tokens a second. The first token comes after 15 ms plus the time to read the prompt; then one completion alone is written at 500 tokens a second.

vLLM batches: one GPU writes up to **Batch slots per GPU** completions at the same time, but every completion in the batch slows the others down. With a full batch, each is written at half speed. A request that finds every slot busy waits. Your model and its working memory fit at most **64 completions at once on one GPU**: keep Batch slots per GPU at 64 or less.

A vLLM card is a pool of GPU servers deployed together (one node pool, one rollout); its **Instances** are its GPUs, each with its own copy of the model. A pool can fail as a whole, for example after a bad driver update. Each GPU costs $734 a month.

## The tests

One test is visible. You can run it as often as you like:

- **A normal day**: the traffic above, at peak.

Two more tests are hidden, and you see only their names: **A busy hour: four times the typing** and **A GPU card is lost**. A real busy hour lasts an hour; the test runs one minute at four times the traffic, long enough for any queue to show. The GPU test allows 2% errors and 98% availability. Your design has to pass all three.
